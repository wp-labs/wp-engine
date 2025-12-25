mod function;
mod pipe;
pub use function::{
    CharsDecode, ExistsChars, FCharsHas, FCharsIn, FCharsNotHas, FDigitHas, FDigitIn, FIpAddrIn,
    FdHas, StubFun,
};
pub use pipe::WplFun;
pub use pipe::WplPipe;
