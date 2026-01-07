use regex::Regex;

fn main() {
    let pr_ref = "puemos/hls-downloader/490";
    let pattern = r"^(?:https://github\.com/)?([^/]+)/([^/]+)/pull/(\d+)$|(?:([^/]+)/([^/#]+))?#?(\d+)$";
    let re = Regex::new(pattern).unwrap();

    if let Some(caps) = re.captures(pr_ref) {
        println!("Match found!");
        for i in 0..caps.len() {
            println!("Group {}: {:?}", i, caps.get(i).map(|m| m.as_str()));
        }
    } else {
        println!("No match found");
    }
}
