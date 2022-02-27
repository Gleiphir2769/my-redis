

pub trait dict {
    fn Put<T>(key: String, val: T) -> i32;
    fn Get<T>(key: String) -> (T, bool);
}