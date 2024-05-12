
//! Simple Json Parser Combinator
//! 
//! Works fine on well-formed input, more testing required for ill-formed json input
//! 
//! Older JSON specs only allowed the top-level element to be an object or an array.  
//! Now any json value is a valid top level element in a json file

#![allow(non_snake_case)] 

/* (IMPORTS) */
use std::collections::HashMap;


/// Enum for various JSON types, with variants for each possible json value
#[derive(Debug)]
pub enum JSON {
    JsNull,             
    JsBool(bool),       
    JsNumber(f32),      
    JsString(String),   
    JsArray(Vec<JSON>), 
    JsObject(HashMap<String, JSON>),
}

// Define Parser trait 
// Left: (remaining unparsed input, reference to matched str) -- Right: Input on which parser failed 
trait Parser<'a, T> { 
    fn parse(&self, input: &'a str) -> Result<(&'a str, T), &'a str>; 
}                                                                     

// Implement parser trait for some generic function F
impl<'a, F, T> Parser<'a, T> for F
where
    F: Fn(&'a str) -> Result<(&'a str, T), &'a str>, 
{
    fn parse(&self, input: &'a str) -> Result<(&'a str, T), &'a str> { 
        self(input)
    }
}


/* (PRIMITIVE COMBINATORS) */

// Function that returns a parser that attempts to match its str against the start of the given input                            
fn str_parser<'a>(s: &'a str) -> impl Parser<'a, &'a str> 
{
    move |input: &'a str|  {    if input.starts_with(s) { 
                                    Ok( (&input[s.len()..], s) ) //If match return shifted input str & matched str
                                } else { 
                                    Err(input)                   //Else return unshifted input str
                                } 
                            }
}


/* (DERIVED COMBINATORS) */

// Sequences 2 parsers, trys the first parser if passes returns that result, otherwise trys the second
fn or<'a, P1, P2, A>(p1: P1, p2: P2) -> impl Parser<'a, A>
where 
    P1: Parser<'a, A>,
    P2: Parser<'a, A>
{   
    move |input: &'a str| { p1.parse(input).or( p2.parse(input) ) }
}

// Sequences 2 parsers, running p1 then p2 and returns the pair of their results only if both succeed
fn product<'a, P1, P2, R1, R2>(p1: P1, p2: P2) -> impl Parser<'a, (R1, R2)>
where
    P1: Parser<'a, R1>,
    P2: Parser<'a, R2>,
{
    move |input| {
        p1.parse(input).and_then(|(next_input, r1)| { //Note: and_then is flatMap
            p2.parse(next_input)
                .map(|(last_input, r2)| (last_input, (r1, r2)))
        })
    }
}

// Parser adapter that matches a quoted string literal 
fn quoted_string_literal<'a, P>(p: P) -> impl Parser<'a, &'a str> 
where 
    P: Parser<'a, &'a str>
{
    move |input| 
        str_parser("\"").parse(input)
            .and_then(|(next_input, _)| { p.parse(next_input) })
                .and_then(|(next_input2, matched)| { 
                    
                    match str_parser("\"").parse(next_input2) {
                        Ok((next, _)) => Ok((next, matched)),          
                        Err(e) => Err(e)
                    }
                }
        )
}

// Parser adapter that matches zero or more instance of a str against a given input
fn zero_or_more<'a, P, A>(p: P) -> impl Parser<'a, Vec<A> >  
where 
    P: Parser<'a, A>
{
    move |input: &'a str| {

        let mut v = vec![];
        let mut inputted: &str = &input; //Is reference to str that gets fed to parser

        while let Ok((next, matches)) = p.parse(inputted) {
            inputted = next;    //"Shift" forward str to be fed to parser if parser correctly parsed str
            v.push(matches);
        }

        Ok((inputted, v)) //Return all unparsed input and the input on the original str that got parsed
    }
}

// Function that returns the left value from a parser with a pair result
fn left<'a, P, A, B>(p: P) -> impl Parser<'a, A> 
where 
    P: Parser<'a, (A, B)>,
{   
    move |input: &'a str| {
        match p.parse(input) {
            Ok((s, (a, _b))) => Ok((s, a)), 
            Err(e)  =>  Err(e)
        }
    }
}

// Function that returns the right value from a parser with a pair result
fn right<'a, P, A, B>(p: P) -> impl Parser<'a, B> 
where 
    P: Parser<'a, (A, B)>,
{   
    move |input: &'a str| {
        match p.parse(input) {
            Ok((s, (_a,b))) => Ok((s, b)), 
            Err(e)  =>  Err(e)
        }
    }
}


/* (GENERAL PARSERS) */

// Function to match whitespace
fn match_whitespace_char<'a>(input: &'a str) -> Result<(&'a str, &'a str), &'a str> 
{
    let mut n = 0;
    let mut chars = input.chars();

    while let Some(ch) = chars.next()  {
        if !ch.is_whitespace() { break; }    //
        n += 1;
    }
    Ok( (&input[n..], &input[..n]) )  //Should return all the space or just eat them ie.  Ok( (&input[n..], "") )
}

// Function to match ascii digit characters
fn match_digit_chars<'a>(input: &'a str) -> Result<(&'a str, &'a str), &'a str> 
{
    //Idea: Probably ought to check if the number you are try to fit is larger than type capacity
    let not_a_digit = 'a';// a is used as default non-ascii digit value
    let mut n = 0;
    let mut chars = input.chars();
    let mut ch = chars.next().unwrap_or(not_a_digit); 

    while ch.is_ascii_digit() {
        n += 1;
        ch = chars.next().unwrap_or(not_a_digit);
    }

    if n == 0 {  //Is not digit, so return err
        return Err(input);
    
    }else if ch != '.' {  //Else if n > 0 then early return for integer (ie. no decimal place)
        return Ok( (&input[n..] , &input[..n]) );
    }
    
    n += 1; //Increment for decimal character

    //Count all diigts after the decimal
    while let Some(ch) = chars.next()  {
        if !ch.is_ascii_digit() { break; }
        n += 1;
    }

    Ok( (&input[n..] , &input[..n]) ) //Return shifted input json str and float str
   
}

// Function to match alphanumberic & space characters (Does JSON allow punction chars in keys & values?)
// This function is essientally for matching the key & values of string literals in json input
fn match_until_double_quote<'a>(input: &'a str) -> Result<(&'a str, &'a str), &'a str> 
{
    let mut n = 0;
    let mut chars = input.chars();

    while let Some(ch) = chars.next()  {
        //if !ch.is_alphanumeric() && !ch.is_whitespace() { break; } 
        if ch == '\"' { break; }         // This is essentially the behaviour we want
        n += 1;
    }
    Ok( (&input[n..] , &input[..n]) )
}

/* (JSON PARSERS) */

// Parser for JsNull
fn json_null<'a>(json_input: &'a str) ->  Result<(&'a str, JSON), &'a str> 
{
    match str_parser("null").parse(json_input) { 
        Ok((next_input, _)) =>  Ok((next_input, JSON::JsNull)),
        Err(e)   => return Err(e) //Return input str where parser failed
    }
}

// Parser for JsBool
fn json_bool<'a>(json_input: &'a str) -> Result<(&'a str, JSON), &'a str>  
{
    //Try parsing the input json for either true or false
    let result = or(str_parser("true"), str_parser("false")).parse(json_input);

    match result { 
        Ok((next_input, "true"))  => Ok((next_input, JSON::JsBool(true))),
        Ok((next_input, "false")) => Ok((next_input, JSON::JsBool(false))),
        Ok(_) => unimplemented!(), // This should never happen but is necessary for exhaustive pattern
        Err(s)  => Err(s)    // Return input str where parser failed
    }
}

// Parser for JsNumber
fn json_number<'a>(json_input: &'a str) -> Result<(&'a str, JSON), &'a str> 
{
    match_digit_chars.parse(json_input)
                     .map( |(next_input, literal)|
                                (next_input, JSON::JsNumber( literal.parse::<f32>().unwrap() ))
                         )
} 

// Parser for JsString
fn json_string<'a>(json_input: &'a str) -> Result<(&'a str, JSON), &'a str> 
{
    quoted_string_literal(match_until_double_quote).parse(json_input)
                                                            .map( |(next_input, literal)| 
                                                                        (next_input, JSON::JsString(literal.to_string()))
                                                                )
} 

// Parser for JsArray
fn json_array<'a>(json_input: &'a str) -> Result<(&'a str, JSON), &'a str> 
{   
    str_parser("[").parse(json_input) //Match opening bracket for json array and then ...
        .and_then(|(next_input, _)| { 

            // Create parser to match some ammount of whitespace followed by either a comma or a closing bracket
            let closing_char = or(str_parser(","), str_parser("]"));
            let whitespace_closing_char = product(match_whitespace_char, closing_char);
            
            // Create a parser that matches some whitespace then a json value then a closing char but only keeps the json value
            let json_value = right( product(match_whitespace_char, parse_json) );
            let json_elements = left( product(json_value, whitespace_closing_char) );

            // Match zero or more json elements 
            match zero_or_more( json_elements ).parse(next_input) {
                Ok((last_input, vec_json)) => Ok((last_input, JSON::JsArray( vec_json ))), 
                Err(e) => Err(e) //Return input str where parser failed
            }
        })
          
}

// Parser for JsObject
fn json_object<'a>(json_input: &'a str) -> Result<(&'a str, JSON), &'a str> 
{
    str_parser("{").parse(json_input) //Match opening curly brace for json object and then ...
        .and_then(|(next_input, _)| { 

            // Create parser to match some amount of whitespace followed by either a comma or a closing curly brace
            let closing_char = or(str_parser(","), str_parser("}"));
            let whitespace_closing_char = product(match_whitespace_char, closing_char);

            // Create a parser that matches some whitespace then an identifier (ie. key) then some more whitespace
            // then a seperator (ie. :) then more whitespace then a json value. But only keeps the json value
            let key = right( product(match_whitespace_char, quoted_string_literal(match_until_double_quote)) );
            let separator = product(match_whitespace_char, str_parser(":"));
            let json_value = right( product(match_whitespace_char, parse_json) );

            // Combine above parsers in order to get required key,value pairs
            let key_sep = left( product(key, separator) );
            let json_val_closing_ch = left( product(json_value, whitespace_closing_char) );

            let json_elements = product(key_sep, json_val_closing_ch);
            
            // Match zero or more json elements 
            match zero_or_more( json_elements ).parse(next_input) {
                Ok((last_input, vec_json)) => { 

                    let mut hashmap_json: HashMap<String, JSON> = HashMap::new();

                    //Create hashmap from vec of json pairs
                    for (s, js) in vec_json { 
                        hashmap_json.insert(String::from(s), js);
                    }

                    Ok((last_input, JSON::JsObject( hashmap_json ))) 
                },   
                Err(e) => Err(e)                 
            }
        })
}


/// Function returns either a reference to the end of the input string along with the parsed JSON 
/// or else returns the input str at the point at which the parser failed.

/// # Arguments
/// Is meant to be called with a string containing the json input to be parsed.

/// # Lifetimes 
/// Since in either case of the result a reference to the input is returned. 
/// Therefore, the input must live at least as long as the output.

// If the function returns a Result, describing the kinds of errors that might occur and what conditions might cause those
// errors to be returned can be helpful to callers so they can write code to handle the different kinds of errors in different ways. 
/// # Errors 
/// On error, the function returns the input str at the point at which the parser failed

// Only necessary if the function contains an unsafe block
// # Safety 

// The scenarios in which the function being documented could panic.
// # Panics 

// Show example use cases of public functions
/// # Examples
/// ```
/// //Compares JSON output as strings
/// let arg = r#"{ "FirstName" : "Michael", "Age" : 33 }"#;
/// let output = Parser::parse_json(arg);
/// let result = format!("{:?}", output);
/// 
/// let answer = r#"Ok(("", JsObject({"FirstName": JsString("Michael"), "Age": JsNumber(33.0)})))"#;
/// assert_eq!(result, answer);
/// ```

// Trys to match every possible json value (ie. null, bool, number, string, array, object)
// Returns first correct match or else error
pub fn parse_json<'a>(json_input: &'a str) -> Result<(&'a str, JSON), &'a str>//Result<JSON, &'a str> // impl Parser<JSON>
{
    //Older JSON specs only allowed the top-level element to be an object or an array.  
    //Now any json value is a valid top level element in a json file

    //Jump table to all possible json parsers
    let json_parsers: Vec< Box< fn(&str) -> Result<(&str, JSON), &str >>> 
                        = vec![ Box::new(json_null), Box::new(json_bool),
                                Box::new(json_string), Box::new(json_number),
                                Box::new(json_array), Box::new(json_object) ];

    //Try to parse input as every possible json value
    for func_ptr in json_parsers {
                                                // Trim to remove leading and trailing whitespace
        if let Ok((next_input, json)) = func_ptr.parse(json_input.trim()) {
            return Ok((next_input, json));      //If successfully parsed then next_input should be empty
        }
    }

    //If unable to parse json value return input that parser failed on
    Err(json_input)  
}


/* (TESTS) */
#[cfg(test)]
mod tests {
    use super::*;

    #[test] 
    fn test_str_parser() 
    {
        let parse_hello = str_parser("Hello");

        assert_eq!( Ok(("", "Hello")), parse_hello.parse("Hello") );
        assert_eq!( Err("Yello"), parse_hello.parse("Yello") );
        assert_eq!( Ok((" Jello", "Hello")), parse_hello.parse("Hello Jello"));
    }

    #[test]
    fn test_parser_or()
    {
        let parse_hello = str_parser("Hello"); 
        let parse_goodbye = str_parser("Goodbye");
        let parse_or = or(parse_hello, parse_goodbye);

        assert_eq!( Err(""), parse_or.parse("") );
        assert_eq!( Ok(("", "Hello")), parse_or.parse("Hello"));                    //P1 succeeds
        assert_eq!( Ok(("", "Goodbye")), parse_or.parse("Goodbye"));                //P2 succeeds
        assert_eq!( Ok((" Goodbye", "Hello")), parse_or.parse("Hello Goodbye"));    //Both succeed
    }

    #[test]
    fn test_parser_product()
    {
        let p1 = product( str_parser("Goodbye"), str_parser(" Adieu"));
        let p2 = product( str_parser("Hello"), str_parser(" Adieu"));
        let p3 = product( str_parser("Hello"), str_parser(" Goodbye"));

        assert_eq!( Err(""), p1.parse("") );
        assert_eq!( Err(""), p2.parse("") );
        assert_eq!( Err(""), p3.parse("") );

        assert_eq!( Err("Hello Adieu"), p1.parse("Hello Adieu"));                   //P1 fails
        assert_eq!( Err(" Goodbye"), p2.parse("Hello Goodbye"));                    //P2 fails
        assert_eq!( Ok( ("", ("Hello", " Goodbye"))), p3.parse("Hello Goodbye"));   //Both succeed
    }

    #[test]
    fn test_parser_quoted_str_literal()
    {
        let parse_quoted_hello = quoted_string_literal(str_parser("Hello")); 

        assert_eq!( Err(""), parse_quoted_hello.parse("") );
        assert_eq!( Err(""), parse_quoted_hello.parse("\"Hello") ); //Err returns empty str since fails to match missing closing quote
        assert_eq!( Err("Hello\""), parse_quoted_hello.parse("Hello\"") );

        assert_eq!( Ok(("", "Hello")), parse_quoted_hello.parse("\"Hello\""));
    }


    #[test]
    fn test_zero_or_more()
    {
        let p1 = zero_or_more(str_parser(" "));
        let p2 = zero_or_more(str_parser("ab"));
      
        assert_eq!( Ok(("", vec![])), p1.parse("") );                       //Successfully match 0 spaces (Note: Returns empty vec)
        assert_eq!( Ok(("", vec![" "])), p1.parse(" ") );                   //Successfully match single space
        assert_eq!( Ok(("", vec![" ", " ", " ", " "])), p1.parse("    ") ); //Successfully match 4 spaces
        assert_ne!( Ok(("", vec![])), p1.parse(" ") );  //Should this match?

        assert_eq!( Ok(("", vec![])), p2.parse("") );                           //Successfully match  (Note: Returns empty vec)
        assert_eq!( Ok(("", vec!["ab"])), p2.parse("ab") );                     //Successfully match single 
        assert_eq!( Ok(("", vec!["ab", "ab", "ab", "ab"])), p2.parse("abababab") ); //Successfully match 4 
    }

    #[test]
    fn test_left()
    {
        let parser = product( str_parser("Hello"), str_parser(" Goodbye"));
        let p = left( parser );

        assert_eq!( Err(""), p.parse("") );
        assert_eq!( Ok(("", "Hello")), p.parse("Hello Goodbye") );           
        assert_eq!( Ok((" Again", "Hello")), p.parse("Hello Goodbye Again") );           
    }

    #[test]
    fn test_right()
    {
        let parser = product( str_parser("Hello"), str_parser(" Goodbye"));
        let p = right( parser );

        assert_eq!( Err(""), p.parse("") );
        assert_eq!( Ok(("", " Goodbye")), p.parse("Hello Goodbye") );           
        assert_eq!( Ok((" Again", " Goodbye")), p.parse("Hello Goodbye Again") ); 
    }

    #[test]
    fn test_match_whitespace_char()
    {
        assert_eq!( Ok(("abc", "")), match_whitespace_char("abc") );  
        assert_eq!( Ok(("abc", " ")), match_whitespace_char(" abc") );    

        assert_eq!( Ok(("", "")), match_whitespace_char("") );          //Successfully match empty string
        assert_eq!( Ok(("", "\n")), match_whitespace_char("\n") );      //Successfully match newline char
        assert_eq!( Ok(("", "\t")), match_whitespace_char("\t") );      //Successfully match tab char          
        assert_eq!( Ok(("", "    ")), match_whitespace_char("    ") );  //Successfully match 4 spaces
    }

    #[test]
    fn test_match_until_double_quote() 
    {
        assert_eq!( Ok(("", "")), match_until_double_quote("") );  
        assert_eq!( Ok(("\"abc", "")), match_until_double_quote("\"abc") );  
        assert_eq!( Ok(("\"", "abc")), match_until_double_quote("abc\"") );  
        assert_eq!( Ok(("\" 456", "abc 123 ")), match_until_double_quote("abc 123 \" 456") );  
        assert_eq!( Ok(("\" 456", "abc -+= 123 ")), match_until_double_quote("abc -+= 123 \" 456") ); 
    }

    #[test]
    fn test_match_digits_chars()
    {
        assert_eq!( Err("abc"), match_digit_chars("abc") );    
        assert_eq!( Err(""), match_digit_chars("") );                 //Fails to match empty string

        assert_eq!( Ok(("", "123")), match_digit_chars("123") );      //Successfully match integer
        assert_eq!( Ok(("", "12.34")), match_digit_chars("12.34") );  //Successfully match float   
    }
}