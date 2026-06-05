# Project Title
BT ANY TYPE

## Description
Basic HashMap to store any data type in a thread-safe manner with zero dependencies

## Usage
```
    use bt_anytype_map::anytype::AnyTypeMap;
    let state = AnyTypeMap::default();
    store.insert(String::from("hello"));
    let hello: String = store.get().unwrap(); 
    assert_eq!(hello.as_str(), "hello");                       
```

## Version History
* 0.1.0
    * Initial Release

## License
GPL-3.0-only