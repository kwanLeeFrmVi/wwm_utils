#[cfg(test)]
mod tests {
    use crate::core;
    use crate::prelude::*;

    #[test]
    fn test_decompress_file() {
        const ZONE_PATH: &str = r"C:\Users\m1zu\Desktop";

        let file_name = "translate_words_map_zh_cn";
        let file_path = Path::new(ZONE_PATH).join(file_name);

        core::unpack_map(file_path);
        core::pack_map(r"D:\_WorkSpace\Main\wwm_utils\output\translate_words_map_zh_cn");
    }

    #[test]
    fn test_decompress_file_v2() {
        const ZONE_PATH: &str = r"C:\Users\m1zu\Desktop\2";

        let file_name = "translate_words_map_en";
        let file_path = Path::new(ZONE_PATH).join(file_name);

        core::unpack_map(file_path);
    }
}
