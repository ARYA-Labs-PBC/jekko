use super::*;

pub(crate) fn value(args: &[String], flag: &str) -> Option<String> {
    args.windows(2)
        .find(|window| window[0] == flag)
        .map(|window| window[1].clone())
}

pub(crate) fn path_value(args: &[String], flag: &str) -> Option<PathBuf> {
    value(args, flag).map(PathBuf::from)
}

pub(crate) fn usize_value(args: &[String], flag: &str, default: usize) -> usize {
    value(args, flag)
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(default)
}

pub(crate) fn u64_value(args: &[String], flag: &str, default: u64) -> u64 {
    value(args, flag)
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(default)
}

pub(crate) fn i32_value(args: &[String], flag: &str, default: i32) -> i32 {
    value(args, flag)
        .and_then(|value| value.parse::<i32>().ok())
        .unwrap_or(default)
}

pub(crate) fn f64_value(args: &[String], flag: &str, default: f64) -> f64 {
    value(args, flag)
        .and_then(|value| value.parse::<f64>().ok())
        .unwrap_or(default)
}
