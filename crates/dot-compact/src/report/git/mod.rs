mod log;
mod status;

pub(crate) use self::log::compact as log;
pub(crate) use self::status::compact as status;

const GIT: &str = "git";
