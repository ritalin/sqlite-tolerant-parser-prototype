use phf::phf_map;
use sqlite_parser_proto::ScanPattern;

pub static LEXME_SCAN_RULE: phf::Map<char, &'static [ScanPattern]> = phf_map!{
    's' => &[
        ScanPattern { id: 201u32, pattern: "SELECT", len: 5 },
    ],
};

pub static REGEX_SCAN_RULE: &[ScanPattern] = &[
    ScanPattern { id: 330u32, pattern: r"\w+", len: 2 },
];