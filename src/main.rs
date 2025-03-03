use graphics_tutorial1_lib::show_strucutre;

fn main() {
    // get the file name from the first argument passed to the program
    let args: Vec<String> = std::env::args().collect();
    let file = &args[1];
    pollster::block_on(show_strucutre(&file));
}
