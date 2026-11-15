// expect: value 42
use std::text::{form_decode, escape_html};
pub fn probe() -> i64 {
    assert!(form_decode("") == "");
    assert!(form_decode("read+file%20name") == "read file name");
    assert!(form_decode("%26%3C%3e%22%27") == "&<>\"'");
    assert!(form_decode("%2B+") == "+ ");
    assert!(form_decode("%") == "%");
    assert!(form_decode("%2") == "%2");
    assert!(form_decode("bad%2Z%tail%") == "bad%2Z%tail%");
    assert!(form_decode("%C3%A9") == "é");
    assert!(form_decode("%FF%41%E2%82") == "�A�");
    assert!(form_decode("%E0%80%80") == "���");
    assert!(form_decode("%2520") == "%20");
    assert!(form_decode("%00") == "\0");
    assert!(form_decode("cat🐈") == "cat🐈");
    assert!(form_decode("a%2fb%2Fc") == "a/b/c");
    assert!(escape_html("") == "");
    assert!(escape_html("\"<&>' π") == "&quot;&lt;&amp;&gt;&#39; π");
    assert!(escape_html("&amp;") == "&amp;amp;");
    assert!(escape_html("🦀\n\0") == "🦀\n\0");
    assert!(escape_html("<tag data=\"x\">&'</tag>") == "&lt;tag data=&quot;x&quot;&gt;&amp;&#39;&lt;/tag&gt;");
    42
}
