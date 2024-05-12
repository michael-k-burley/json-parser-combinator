
#![allow(non_snake_case)]

use std::io::{Read, Write}; //Read file to string
use std::fs::File; //For parser unit test

use Parser::parse_json;

fn main() {

    let file_name = "arr2";
   
    //Input JSON File 
    let mut input_file = File::open(format!("./json_test_samples/input/{file_name}.json")).unwrap();

    //String for json from file
    let mut json_str = String::new();                               

    //Read file into string
    input_file.read_to_string(&mut json_str).unwrap();


    //Pretty print out parsed json output 
    match parse_json(&json_str) {

        // If a portion of the json str remains (besides trailing newlines), then the parser failed at the start of the remaining json str
        Ok((_unparsed, result)) => {  

            //Output JSON file
            let mut output_file = File::create(format!("./json_test_samples/output/{file_name}.txt")).unwrap();
            
            //Convert to string
            let result_str =  format!("{result:#?}"); 

            //Write string to file
            output_file.write_all(result_str.as_bytes()).unwrap();
        } 
        Err(e) => println!("ERROR: \n {e:?}") // Only throws error if unable to parse top level json element
    }
    
    println!("END!");
}

