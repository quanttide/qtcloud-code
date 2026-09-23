fn parse_age(input: &str) -> Result<f64, String> {
    let age: f64 = input.trim().parse().map_err(|e| e.to_string())?;
    if age > 150.0 {
        panic!("age out of range");
    }
    let raw = 42i32;
    let as_f64 = raw as f64;
    let _dangling = unsafe { std::ptr::read(&raw as *const i32) };
    if input.is_empty() {
        return Err("invalid".to_string());
    }
    Ok(age)
}
