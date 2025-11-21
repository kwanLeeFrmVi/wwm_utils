use crate::prelude::*;

pub trait SeekReadExt: Seek + Read + StructReadExt + ArrayReadExt + StringReadExt {
    fn read_struct_at<S: Copy + 'static>(&mut self, pos: u64) -> Result<S, io::Error> {
        self.seek(SeekFrom::Start(pos))?;
        self.read_struct::<S>()
    }

    fn read_array_at<R: Copy + 'static>(
        &mut self,
        pos: u64,
        length: usize,
    ) -> Result<Vec<R>, io::Error> {
        self.seek(SeekFrom::Start(pos))?;
        self.read_array(length)
    }

    fn read_sized_string_at(
        &mut self,
        pos: u64,
        size: usize,
        null_terminator: bool,
    ) -> Result<String, io::Error> {
        self.seek(SeekFrom::Start(pos))?;
        self.read_sized_string(size, null_terminator)
    }

    fn remaining_size(&mut self) -> Result<u64, io::Error> {
        let cur = self.stream_position()?;
        let length = self.seek(SeekFrom::End(0))?;
        let remaining = length - cur;
        self.seek(SeekFrom::Start(cur))?;

        Ok(remaining)
    }
}

impl<T> SeekReadExt for T where T: Seek + Read + StructReadExt + ArrayReadExt + StringReadExt {}
