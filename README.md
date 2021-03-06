[![crates.io](https://img.shields.io/crates/v/glade-bindgen.svg)](https://crates.io/crates/glade-bindgen)

You can generate glade bind with build script

```
#Cargo.toml
[build-dependencies]
glade-bindgen = "1.1"
```


```
#build.rs
fn main() {
    glade_bindgen::generate_bind_build_script("src/path_to_glade_files", true);
    //disable generating get() method if false===========================^^^^
}
```
For example, if you have `example.glade` at `src/path_to_glade_files`,
it will generate struct `path_to_glade_files::Example`

```
#src/main.rs
pub mod path_to_glade_files; //you need to include module

use path_to_glade_files::Example;

fn main() {
    let button: &gtk::Button = &Example::get().your_button_id;
    //you can use editor's autocompletion here ^^^^^^^^^^^^
}
```

You can also use it as owned value
```
let example = Example::new();
let button: &gtk::Button = &example.your_button_id;
//you can use editor's autocompletion here ^^^^^^
```
