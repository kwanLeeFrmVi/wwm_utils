use crate::prelude::*;
use crate::structs::*;

const OUTPUT_DIR: &str = "output";
const TABLE_DIR: &str = "tables";
const TEXT_DIR: &str = "text";
const MODIFIED_DIR: &str = "modified";
const MERGED_DIR: &str = "merged";
const ENTRIES_FILE: &str = "entries.json";
const ENTRIES_PER_SHARD: usize = 265;

pub fn unpack_map<P: AsRef<Path>>(path: P) {
    let path = path.as_ref();

    println!("Unpacking: `{}`", path.display());

    let mut reader = File::open(path).unwrap().buffer_read();

    let header = reader.read_struct::<MapHeader>().unwrap();

    let file_stem = path.file_stem().unwrap().to_str().unwrap();
    let output_dir = Path::new(OUTPUT_DIR).join(file_stem);

    // Remove existing output (file or directory) before unpacking
    if output_dir.exists() {
        if output_dir.is_dir() {
            fs::remove_dir_all(&output_dir).unwrap();
        } else {
            fs::remove_file(&output_dir).unwrap();
        }
    }
    fs::create_dir_all(&output_dir).unwrap();

    let seek_table = reader
        .read_array::<u32>((header.entry_count + 1) as usize)
        .unwrap();

    let start_pos = reader.stream_position().unwrap();
    // println!("start_pos: {start_pos}");

    let mut string_map = HashMap::<u64, String>::new();

    for (i, &offset) in seek_table.iter().enumerate() {
        if i == seek_table.len() - 1 {
            continue;
        }

        let entry_pos = start_pos + offset as u64;
        // println!("entry_pos: {entry_pos}");

        let entry_header = reader.read_struct_at::<BlockHeader>(entry_pos).unwrap();

        let compressed_data = reader
            .read_array::<u8>(entry_header.compressed_size as usize)
            .unwrap();

        match entry_header.compression_type {
            // zstd compression
            4 => {
                let decoded = zstd::decode_all(compressed_data.as_bytes()).unwrap();
                let mut table_reader = Cursor::new(&decoded);

                if i > 0 {
                    let table_header = table_reader.read_struct::<TableHeader>().unwrap();

                    // Min: (3 + 1(0xFF)) + 16(entry_count + n * 0x80) + 4(padding 0) = 24
                    // Max: (511 + 1(0xFF)) + 16(entry_count) = 528
                    let bucket_size = ((table_header.entry_count + 1) + 16).max(24);
                    let table_buckets =
                        table_reader.read_array::<u8>(bucket_size as usize).unwrap();

                    let mut cur_entry_pos = table_reader.stream_position().unwrap();

                    let table_entries = table_reader
                        .read_array::<TableEntry>(table_header.entry_count as usize)
                        .unwrap();

                    for entry in table_entries {
                        let id = entry.id;

                        if id > 0 && entry.length > 0 {
                            let value_pos = cur_entry_pos + 8 + entry.offset as u64;
                            let byte = table_reader.read_struct_at::<u8>(value_pos).unwrap();

                            if byte != 0xFF {
                                let value = table_reader
                                    .read_sized_string_at(value_pos, entry.length as usize, false)
                                    .unwrap();
                                // println!("{value}");

                                let inserted = string_map.insert(id, value);

                                if inserted.is_some() {
                                    eprintln!("Duplicate string id found: {:#x}", id);
                                }
                            } else {
                                // Map 0xFF entries to empty string
                                string_map.insert(id, String::new());
                            }
                        }

                        cur_entry_pos += size_of::<TableEntry>() as u64;
                    }
                }

                // Save tables
                {
                    let table_dir = output_dir.join(TABLE_DIR);
                    let table_path = table_dir.join(i.to_string());

                    fs::create_dir_all(&table_dir).unwrap();

                    let mut writer = File::create(table_path).unwrap().buffer_write();
                    writer.write_all(&decoded).unwrap();
                }
            }
            _ => {
                eprintln!(
                    "Unknown compression type: {}",
                    entry_header.compression_type
                );
            }
        }
    }

    // Save entries.json (single file with all entries)
    {
        let output_path = Path::new(&output_dir).join(ENTRIES_FILE);

        let mut writer = File::create(&output_path).unwrap().buffer_write();

        // To JSON
        let json = json!(string_map);

        let bytes = serde_json::to_vec_pretty(&json).unwrap();
        writer.write_all(&bytes).unwrap();
    }

    // Save text/ directory with sharded JSON files
    {
        let text_dir = output_dir.join(TEXT_DIR);
        fs::create_dir_all(&text_dir).unwrap();

        let entries: Vec<_> = string_map.iter().collect();
        let shard_count = (entries.len() + ENTRIES_PER_SHARD - 1) / ENTRIES_PER_SHARD;

        for shard_idx in 0..shard_count {
            let start = shard_idx * ENTRIES_PER_SHARD;
            let end = (start + ENTRIES_PER_SHARD).min(entries.len());
            
            let shard_entries: HashMap<&u64, &String> = entries[start..end]
                .iter()
                .map(|(k, v)| (*k, *v))
                .collect();

            let shard_path = text_dir.join(format!("{:05}.json", shard_idx + 1));
            let mut writer = File::create(&shard_path).unwrap().buffer_write();
            
            let json = json!(shard_entries);
            let bytes = serde_json::to_vec_pretty(&json).unwrap();
            writer.write_all(&bytes).unwrap();
        }
    }

    println!("Unpacked: `{}`", path.display());
}

pub fn pack_map<P: AsRef<Path>>(dir_path: P) {
    let dir_path = dir_path.as_ref();

    println!("Packing: `{}`", dir_path.display());

    let entries_path = dir_path.join(ENTRIES_FILE);
    let entries_map = fs::read(entries_path).map_or(Default::default(), |buffer| {
        serde_json::from_slice::<HashMap<u64, String>>(&buffer).unwrap_or_default()
    });

    let table_dir = dir_path.join(TABLE_DIR);

    let mut tables = WalkDir::new(table_dir)
        .into_iter()
        .filter_map(|result| result.ok())
        .filter(|entry| entry.file_type().is_file())
        .filter(|entry| {
            let file_name = entry.file_name().to_str().unwrap_or("");
            entry.file_type().is_file() && !file_name.starts_with(".")
        })
        .collect::<Vec<_>>();

    // Sort the file indices from 0 to n
    tables.sort_unstable_by(|a, b| {
        let a_idx = a.file_name().to_str().unwrap().parse::<usize>().unwrap();
        let b_idx = b.file_name().to_str().unwrap().parse::<usize>().unwrap();

        a_idx.cmp(&b_idx)
    });

    let table_count = tables.len();

    let mut table_buffers = Vec::<Vec<u8>>::new();

    let mut result = Vec::<u8>::new();
    let mut blocks = Vec::<u8>::new();

    let mut seek_table = vec![0u32; table_count + 1];

    let mut entry_offset: u32 = 0;

    for (table_idx, table) in tables.iter().enumerate() {
        let table_path = table.path();

        if !table_path.is_file() {
            continue;
        }

        let table_stem = table_path.file_stem().unwrap().to_str().unwrap();

        let mut reader = File::open(table_path).unwrap().buffer_read();

        let table_buffer = if table_stem == "0" {
            reader.read_array_to_end().unwrap()
        } else {
            let table_header = reader.read_struct::<TableHeader>().unwrap();

            // Min: (3 + 1(0xFF)) + 16(entry_count + n * 0x80) + 4(padding 0) = 24
            // Max: (511 + 1(0xFF)) + 16(entry_count) = 528
            let bucket_size = ((table_header.entry_count + 1) + 16).max(24);
            let table_buckets = reader.read_array::<u8>(bucket_size as usize).unwrap();

            let entries_pos = reader.stream_position().unwrap();
            let mut cur_entry_pos = entries_pos;

            let table_entries = reader
                .read_array::<TableEntry>(table_header.entry_count as usize)
                .unwrap();

            let mut new_entries = table_entries.clone();
            let mut new_values = Vec::<u8>::new();

            for (entry_idx, entry) in table_entries.iter().enumerate() {
                let id = entry.id;

                let new_entry = &mut new_entries[entry_idx];

                if id > 0 && entry.length > 0 {
                    let value_pos = cur_entry_pos + 8 + entry.offset as u64;
                    
                    let offset = (table_entries.len() - entry_idx) * size_of::<TableEntry>() - 8
                        + new_values.len();
                    
                    // Check if ID exists in JSON first
                    if let Some(value) = entries_map.get(&id) {
                         new_entry.offset = offset as u32;
                         
                         if value.is_empty() {
                             // Treat empty string as 0xFF
                             new_entry.length = 1;
                             new_values.push(0xFF);
                         } else {
                             new_entry.length = value.len() as u32;
                             new_values.extend(value.as_bytes());
                         }
                    } else {
                        // Fallback to original
                        let byte = reader.read_struct_at::<u8>(value_pos).unwrap();
                        
                        if byte != 0xFF {
                             let value = reader
                                .read_array_at::<u8>(value_pos, entry.length as usize)
                                .unwrap();
                             
                             new_entry.offset = offset as u32;
                             new_entry.length = value.len() as u32;
                             new_values.extend(value);
                        } else {
                             // Original was 0xFF
                             new_entry.offset = offset as u32;
                             new_entry.length = 1;
                             new_values.push(0xFF);
                        }
                    }
                } else {
                    // Note: Not needed for game reading, just looks good
                    new_entry.offset = 0xFFFFFFFF;
                }

                cur_entry_pos += size_of::<TableEntry>() as u64;
            }

            let buffer = [
                bytemuck::bytes_of(&table_header),
                &table_buckets,
                bytemuck::cast_slice(&new_entries),
                &new_values,
            ]
            .concat();

            buffer
        };

        // Save modified tables for reference
        {
            let modified_dir = dir_path.join(MODIFIED_DIR);
            let modified_path = modified_dir.join(table_idx.to_string());

            fs::create_dir_all(&modified_dir).unwrap();

            let mut writer = File::create(&modified_path).unwrap().buffer_write();
            writer.write_all(&table_buffer).unwrap();
        }

        let table_size = table_buffer.len();
        let table_encoded = zstd::encode_all(table_buffer.as_bytes(), 0).unwrap();

        table_buffers.push(table_buffer);

        let encoded_size = table_encoded.len() as u32;

        seek_table[table_idx] = entry_offset;
        entry_offset += size_of::<BlockHeader>() as u32 + encoded_size;

        // Save header
        let block_header = BlockHeader {
            compression_type: 4,
            compressed_size: encoded_size,
            decompressed_size: table_size as u32,
        };
        blocks.extend(bytemuck::bytes_of(&block_header));
        // Save data
        blocks.extend(table_encoded);
    }

    seek_table[table_count] = entry_offset;

    // Append map file parts
    {
        // Save header
        let map_header = MapHeader {
            magic: 0xDEADBEEF,
            version: 1,
            entry_count: table_count as u32,
        };
        result.extend(bytemuck::bytes_of(&map_header));

        // Save seek table
        result.extend(bytemuck::cast_slice(&seek_table));

        // Save blocks
        result.extend(blocks);
    }

    // Save merged map file
    {
        let file_name = dir_path.file_stem().unwrap().to_str().unwrap();

        let merged_dir = dir_path.join(MERGED_DIR);
        let merged_path = merged_dir.join(file_name);

        fs::create_dir_all(&merged_dir).unwrap();

        let mut writer = File::create(&merged_path).unwrap().buffer_write();
        writer.write_all(&result).unwrap();

        // let test_path = r"D:\wwm\wwm_lite\LocalData\Patch\HD\oversea\locale\translate_words_map_en";
        // fs::copy(merged_path, test_path).unwrap();
    }

    println!("Packed: `{}`", dir_path.display());
}
