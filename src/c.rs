pub mod crs {
    use std::collections::HashMap;

    pub type CRS = String;

    pub fn crs(code: &CRS) -> &'static str {
        let db = HashMap::from([
            ("EDB".to_string(), "Edinburgh Waverley"),
            ("BHG".to_string(), "Bathgate")
        ]);

        db[code]
    }
}
