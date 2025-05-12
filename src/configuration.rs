use qrcode_generator::QrCodeEcc;

// Quick helper for easier to read sizes
macro_rules! size {
    ($val:literal) => {
        $val
    };
    ($val:literal K) => {
        size!($val) * 1024
    };
    ($val:literal M) => {
        size!($val K) * 1024
    };
    ($val:literal G) => {
        size!($val M) * 1024
    };
    ($val:literal T) => {
        size!($val G) * 1024
    };
}

pub const MAX_UPLOAD_SIZE: usize = size!(100 G);

pub const DOWNLOAD_CHUNKSIZE: usize = size!(1 M);

pub const UPLOAD_BUFSIZE: usize = size!(100 M);

pub const QR_CODE_SIZE: usize = 200; // pixels
pub const QR_CODE_ECC: QrCodeEcc = QrCodeEcc::Medium;

pub const EMBED_DESCRIPTION: &str = "making file transfers trivial";

pub const FAVICON: &str = "/static/favicon.ico";
