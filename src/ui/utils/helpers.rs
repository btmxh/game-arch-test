pub fn wrap_if_not_equals<T>(value: T, compare_value: &T) -> Option<T>
where
    T: PartialEq<T>,
{
    (*compare_value != value).then_some(value)
}
