// Determine which values of type Test are exposed from a given module.

use std::fs::File;
use std::io::{BufReader, BufRead};
use io;
use std::path::{Path, PathBuf};
use std::collections::HashSet;

#[derive(Debug)]
pub enum Problem {
    UnexposedTests(String, HashSet<String>),
    MissingModuleDeclaration(PathBuf),
    OpenFileToReadExports(PathBuf, io::Error),
    ReadingFileForExports(PathBuf, io::Error),
    ParseError(PathBuf),
}

pub fn filter_exposing(
    path: &Path,
    tests: &HashSet<String>,
    module_name: &str,
) -> Result<(String, HashSet<String>), Problem> {
    let new_tests: HashSet<String> = match read_exposing(path)? {
        // None for exposed_values means "the module was exposing (..), so keep everything"
        None => tests.clone(),
        // Only keep the tests that were exposed.
        Some(exposed_values) => {
            exposed_values
                .intersection(&tests)
                .cloned()
                .collect::<HashSet<String>>()
        }
    };

    if new_tests.len() < tests.len() {
        Err(Problem::UnexposedTests(
            module_name.to_owned(),
            tests
                .difference(&new_tests)
                .cloned()
                .collect::<HashSet<String>>(),
        ))
    } else {
        Ok((module_name.to_owned(), new_tests))
    }
}

enum ParsedLineResult {
    AllExposed,
    Exposing(HashSet<String>, bool),
}

fn read_exposing(path: &Path) -> Result<Option<HashSet<String>>, Problem> {
    let file = File::open(path).map_err(|err| {
        Problem::OpenFileToReadExports(path.to_path_buf(), err)
    })?;
    let mut reader = BufReader::new(file);
    let mut line = String::new();
    let mut exposing: HashSet<String> = HashSet::new();

    loop {
        reader.read_line(&mut line).map_err(|err| {
            Problem::OpenFileToReadExports(path.to_path_buf(), err)
        })?;

        match parse_line(&line) {
            Ok(ParsedLineResult::AllExposed) => {
                return Ok(None);
            }
            Ok(ParsedLineResult::Exposing(new_exposing, is_done)) => {
                for val in new_exposing {
                    exposing.insert(val);
                }

                if is_done {
                    return Ok(Some(exposing));
                }
            }
            Err(_) => {
                return Err(Problem::ParseError(path.to_path_buf()));
            }
        }
    }
}

fn parse_line(line: &str) -> Result<ParsedLineResult, ()> {
    return Err(());
}

/* Remove all the comments from the line,
   and return whether we are still in a multiline comment or not
*/
fn strip_comments(line: &mut str, is_in_comment: bool) -> bool {
    loop {
        // when we have a single line comment
        if let Some(single_line_comment_index) = line.find("--") {
            if !is_in_comment {
                unsafe {
                    line.slice_mut_unchecked(0, single_line_comment_index);
                }
                continue;
            }
        }

        let block_comment_start = line.find("{-");
        let block_comment_end = line.find("-}");

        match (block_comment_start, block_comment_end) {
            // when there's a start and end
            (Some(start_index), Some(end_index)) => {
                // We know these indices will be okay because we got them from find()
                unsafe {
                    line.slice_mut_unchecked(0, start_index);
                }

                // Subtract start_index because the line just got shorter by that much.
                let dest_index = (end_index + 2) - start_index;
                let line_length = line.len();

                // We know these indices will be okay because we got them from find()
                unsafe {
                    line.slice_mut_unchecked(dest_index, line_length - dest_index);
                }
            }

            // when there's a start, but no end
            (Some(start_index), None) => {
                // We know these indices will be okay because we got them from find()
                unsafe {
                    line.slice_mut_unchecked(0, start_index);
                }

                return true;
            }

            // when there's an end, but no start
            (None, Some(end_index)) => {
                if is_in_comment {
                    let dest_index = end_index + 2;
                    let line_length = line.len();

                    // We know these indices will be okay because we got them from find()
                    unsafe {
                        line.slice_mut_unchecked(dest_index, line_length - dest_index);
                    }
                }

                return false;
            }

            // when there are no block comment chars
            (None, None) => {
                if is_in_comment {
                    // We know these indices will be okay because they're both 0.
                    unsafe {
                        line.slice_mut_unchecked(0, 0);
                    }
                }

                return is_in_comment;
            }
        }
    }
}
//
// var splitExposedFunctions = function(exposingLine) {
//   return exposingLine
//     .substr(0, exposingLine.lastIndexOf(")"))
//     .split(",")
//     .map(str => str.trim())
//     .filter(str => str[0].toLowerCase() === str[0]);
// };
//
// var isAModuleLine = function(line) {
//   return (
//     line.startsWith("module") ||
//     line.startsWith("port module") ||
//     line.startsWith("effect module")
//   );
// };
//
// function Parser() {
//   // if we're currently in a comment
//   var isInComment = false;
//
//   // if the file does not have a module line
//   var isMissingModuleName = false;
//
//   // if we're done parsing
//   var parsingDone = false;
//
//   // if the module line has been read
//   var hasModuleLineBeenRead = false;
//
//   var isReadingModuleName = false;
//   var isReadingExports = false;
//   var isBetweenBrackets = false;
//
//   // functions that have been exposed
//   var exposedFunctions = [];
//   // number of open/closed brackets seen so far
//   var openBracketsSeen = 0;
//   var closedBracketsSeen = 0;
//   // data between exposing brackets
//   var data = "";
//
//   this.parseLine = function(line) {
//     if (parsingDone) return;
//
//     var whereWeUpTo = stripComments(line, isInComment);
//     isInComment = whereWeUpTo.isInComment;
//     line = whereWeUpTo.line.trim();
//
//     if (line.length == 0) return;
//
//     // if we haven't started reading the first line
//     if (!hasModuleLineBeenRead && isAModuleLine(line)) {
//       hasModuleLineBeenRead = true;
//       // drop module from the line
//       line = line.substr(line.indexOf("module") + 7);
//       isReadingModuleName = true;
//
//       if (line.length === 0) return;
//     }
//
//     // if we manage to find content before the module line,
//     // something is wrong - so exit
//     if (!hasModuleLineBeenRead) {
//       isMissingModuleName = true;
//       parsingDone = true;
//       return;
//     }
//
//     // if we are still reading the module line
//     if (isReadingModuleName) {
//       var exposingIndex = line.indexOf("exposing");
//
//       // if we haven't found exposing yet
//       if (exposingIndex === -1) {
//         return;
//       }
//
//       line = line.substr(exposingIndex + 8);
//       isReadingModuleName = false;
//       isReadingExports = true;
//
//       if (line.length === 0) return;
//     }
//
//     // if we are actually reading the exports
//     if (isReadingExports) {
//       var firstBracket = line.indexOf("(");
//       if (firstBracket === -1) return;
//
//       openBracketsSeen += 1;
//       isReadingExports = false;
//       isBetweenBrackets = true;
//       line = line.substr(firstBracket + 1);
//     }
//
//     // if we're before the final bracket
//     if (isBetweenBrackets) {
//       var newOpenBracketsSeen = line.split("(").length;
//       var newCloseBracketsSeen = line.split(")").length;
//
//       closedBracketsSeen += newCloseBracketsSeen;
//       openBracketsSeen += newOpenBracketsSeen;
//
//       data += line;
//
//       if (closedBracketsSeen === openBracketsSeen) {
//         exposedFunctions = splitExposedFunctions(data);
//         parsingDone = true;
//       }
//     }
//   };
//
//   this.isDoneReading = function() {
//     return parsingDone;
//   };
//
//   this.getExposing = function() {
//     return exposedFunctions;
//   };
//
//   this.getIsMissingModuleName = function() {
//     return isMissingModuleName;
//   };
//
//   return this;
// }
// }
