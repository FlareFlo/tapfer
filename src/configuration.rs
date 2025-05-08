// Quick helper for easier to read sizes
macro_rules! size {
    ($val:literal K) => {
        ($val as usize) * 1024
    };
    ($val:literal M) => {
        ($val as usize) * 1024 * 1024
    };
    ($val:literal G) => {
        ($val as usize) * 1024 * 1024 * 1024
    };
    ($val:literal T) => {
        ($val as usize) * 1024 * 1024 * 1024 * 1024
    };
}

pub const MAX_UPLOAD_SIZE: usize = size!(100 G);

pub const DOWNLOAD_CHUNKSIZE: usize = size!(1 M);
