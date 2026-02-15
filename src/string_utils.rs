//! The process with &str, String and Option<String> is so important.
//! 

pub trait IntoOpStr {
    fn into_op_str(self) -> Option<String>;
}

impl IntoOpStr for &str {
    fn into_op_str(self) -> Option<String> {
        Some(self.to_string())
    }
}

impl IntoOpStr for String {
    fn into_op_str(self) -> Option<String> {
        Some(self)
    }
}

impl IntoOpStr for Option<String> {
    fn into_op_str(self) -> Option<String> {
        self
    }
}

pub fn _convert<T: IntoOpStr>(value: T) -> Option<String> {
    value.into_op_str()
}


