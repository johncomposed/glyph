use pmndrs_glyph_font_baker::{BakeDescriptorV0, BakeErrorCode, abi_json, bake_font};

const INTER: &[u8] =
    include_bytes!("../../../../../benches/fixtures/fonts/inter-v4.1/Inter-Regular.ttf");

#[test]
fn generated_abi_is_valid_and_names_the_public_exports() {
    let abi: serde_json::Value = serde_json::from_str(abi_json()).expect("valid ABI JSON");

    assert_eq!(abi["name"], "pmndrs-glyph-font-baker");
    assert_eq!(abi["version"], 0);
    assert_eq!(abi["versions"]["harfrust"], "0.12.0");
    assert_eq!(
        abi["versions"]["harfrustCommit"],
        "60b28ea22b5261710018d69c168a762bcb28794c"
    );
    assert_eq!(abi["versions"]["harfbuzzReference"], "13.0.0");
    assert_eq!(abi["versions"]["unicode"], "17.0.0");
    assert_eq!(abi["versions"]["gltfSpec"], "2.0");
    assert_eq!(abi["versions"]["binaryen"], "129.0.0");
    assert_eq!(abi["pointerWidth"], 32);
    assert_eq!(abi["functions"]["bake"]["export"], "pmndrs_font_baker_bake");
    assert_eq!(
        abi["functions"]["prepare"]["export"],
        "pmndrs_font_baker_prepare"
    );
    assert_eq!(
        abi["functions"]["inspect"]["export"],
        "pmndrs_font_baker_inspect"
    );
    assert_eq!(abi["response"]["payloadOffset"], 16);
}

#[test]
fn public_api_rejects_an_unknown_descriptor_version_before_font_parsing() {
    let error = bake_font(
        &[],
        BakeDescriptorV0 {
            format_version: 1,
            font_face_index: 0,
            variation: None,
        },
    )
    .expect_err("descriptor version 1 must be rejected");

    assert_eq!(error.code, BakeErrorCode::InvalidDescriptor);
}

#[test]
fn public_api_returns_a_structured_error_for_invalid_font_bytes() {
    let error = bake_font(&[0, 1, 2, 3], BakeDescriptorV0::new(0))
        .expect_err("invalid font bytes must be rejected");

    assert_eq!(error.code, BakeErrorCode::InvalidFont);
    assert!(!error.message.is_empty());
}

#[test]
fn public_api_rejects_webfont_envelopes_before_sfnt_parsing() {
    for signature in [b"wOFF", b"wOF2"] {
        let error = bake_font(signature, BakeDescriptorV0::new(0))
            .expect_err("webfont containers must be decoded by the host");
        assert_eq!(error.code, BakeErrorCode::UnsupportedContainer);
    }
}

#[test]
fn public_api_validates_required_and_unsupported_shaping_tables() {
    let mut missing_os2 = INTER.to_vec();
    let os2 = table_record(&missing_os2, *b"OS/2");
    missing_os2[os2 + 12..os2 + 16].copy_from_slice(&0_u32.to_be_bytes());
    assert_eq!(
        bake_font(&missing_os2, BakeDescriptorV0::new(0))
            .expect_err("an empty required table must fail")
            .code,
        BakeErrorCode::MissingTable,
    );

    assert_eq!(
        bake_font(
            INTER,
            BakeDescriptorV0::new(0).with_variation([("wght".to_owned(), 700.0)])
        )
        .expect_err("axes on a static font must fail")
        .code,
        BakeErrorCode::InvalidDescriptor,
    );

    let mut aat = INTER.to_vec();
    let name = table_record(&aat, *b"name");
    aat[name..name + 4].copy_from_slice(b"morx");
    assert_eq!(
        bake_font(&aat, BakeDescriptorV0::new(0))
            .expect_err("an AAT shaping table must fail")
            .code,
        BakeErrorCode::UnsupportedShapingSystem,
    );
}

#[test]
fn public_api_selects_one_collection_face_and_rejects_an_unknown_index() {
    let collection = two_face_collection(INTER);
    let first = bake_font(&collection, BakeDescriptorV0::new(0)).expect("first TTC face");
    let second = bake_font(&collection, BakeDescriptorV0::new(1)).expect("second TTC face");

    assert_eq!(first.artifacts[0].id, second.artifacts[0].id);
    assert_ne!(
        first.artifacts[0].fingerprint,
        second.artifacts[0].fingerprint
    );
    assert_eq!(
        bake_font(&collection, BakeDescriptorV0::new(2))
            .expect_err("TTC face index is bounds checked")
            .code,
        BakeErrorCode::InvalidFont,
    );
}

fn table_record(font: &[u8], wanted: [u8; 4]) -> usize {
    let count = u16::from_be_bytes(font[4..6].try_into().expect("table count"));
    (0..usize::from(count))
        .map(|index| 12 + index * 16)
        .find(|record| font[*record..*record + 4] == wanted)
        .expect("fixture table")
}

fn two_face_collection(font: &[u8]) -> Vec<u8> {
    let header_len = 20_usize;
    let first_offset = header_len;
    let second_offset = first_offset + font.len().next_multiple_of(4);
    let mut collection = vec![0_u8; second_offset + font.len()];
    collection[0..4].copy_from_slice(b"ttcf");
    collection[4..8].copy_from_slice(&0x0001_0000_u32.to_be_bytes());
    collection[8..12].copy_from_slice(&2_u32.to_be_bytes());
    collection[12..16].copy_from_slice(&(first_offset as u32).to_be_bytes());
    collection[16..20].copy_from_slice(&(second_offset as u32).to_be_bytes());
    for face_offset in [first_offset, second_offset] {
        collection[face_offset..face_offset + font.len()].copy_from_slice(font);
        let count = u16::from_be_bytes(font[4..6].try_into().expect("table count"));
        for index in 0..usize::from(count) {
            let record = 12 + index * 16;
            let source_offset = u32::from_be_bytes(
                font[record + 8..record + 12]
                    .try_into()
                    .expect("table offset"),
            );
            let collection_offset = (face_offset as u32)
                .checked_add(source_offset)
                .expect("fixture collection offset");
            collection[face_offset + record + 8..face_offset + record + 12]
                .copy_from_slice(&collection_offset.to_be_bytes());
        }
    }
    collection
}

#[test]
fn baked_metrics_carry_underline_and_strikeout_values() {
    let result = bake_font(INTER, BakeDescriptorV0::new(0)).expect("Inter must bake");
    let glb = &result
        .artifacts
        .iter()
        .find(|artifact| artifact.role == "font")
        .expect("font artifact")
        .bytes;

    assert_eq!(&glb[0..4], b"glTF");
    let json_length = u32::from_le_bytes(glb[12..16].try_into().expect("chunk length")) as usize;
    assert_eq!(&glb[16..20], b"JSON");
    let document: serde_json::Value =
        serde_json::from_slice(&glb[20..20 + json_length]).expect("GLB JSON chunk");
    let metrics = &document["extensions"]["PMNDRS_font"]["metrics"];

    // Independently parsed from Inter-Regular 4.1: post.underlinePosition/Thickness
    // and OS/2.yStrikeoutPosition/Size at 2048 units per em.
    assert_eq!(metrics["unitsPerEm"], 2048);
    assert_eq!(metrics["underlinePosition"], -348);
    assert_eq!(metrics["underlineThickness"], 140);
    assert_eq!(metrics["strikeoutPosition"], 671);
    assert_eq!(metrics["strikeoutSize"], 140);
}

const OXANIUM: &[u8] =
    include_bytes!("../../../../../benches/fixtures/fonts/oxanium-wght/Oxanium[wght].ttf");

#[test]
fn variable_fonts_bake_to_one_pinned_instance() {
    let default = bake_font(OXANIUM, BakeDescriptorV0::new(0)).expect("default instance");
    let semibold = bake_font(
        OXANIUM,
        BakeDescriptorV0::new(0).with_variation([("wght".to_owned(), 700.0)]),
    )
    .expect("wght=700 instance");
    let clamped = bake_font(
        OXANIUM,
        BakeDescriptorV0::new(0).with_variation([("wght".to_owned(), 900.0)]),
    )
    .expect("wght=900 clamps to the axis maximum");
    let maximum = bake_font(
        OXANIUM,
        BakeDescriptorV0::new(0).with_variation([("wght".to_owned(), 800.0)]),
    )
    .expect("wght=800 instance");

    let tables = |result: &pmndrs_glyph_font_baker::BakeResultV0| {
        result
            .report
            .shared
            .shaping
            .tables
            .iter()
            .map(|table| table.tag.clone())
            .collect::<Vec<_>>()
    };
    assert_eq!(
        tables(&semibold),
        [
            "GDEF", "GPOS", "GSUB", "HVAR", "OS/2", "avar", "cmap", "fvar", "head", "hhea", "hmtx",
            "maxp"
        ]
    );
    assert_eq!(tables(&default), tables(&semibold));
    assert_ne!(default.artifacts[0].id, semibold.artifacts[0].id);
    assert_eq!(clamped.artifacts[0].id, maximum.artifacts[0].id);
    assert_ne!(clamped.artifacts[0].id, semibold.artifacts[0].id);

    let document = |result: &pmndrs_glyph_font_baker::BakeResultV0| -> serde_json::Value {
        let bytes = &result.artifacts[0].bytes;
        let json_len = u32::from_le_bytes(bytes[12..16].try_into().unwrap()) as usize;
        serde_json::from_slice(&bytes[20..20 + json_len]).unwrap()
    };
    let variation = |result| document(result)["extensions"]["PMNDRS_font"]["variation"].clone();
    assert_eq!(
        variation(&default),
        serde_json::json!({ "axes": { "wght": 200.0 }, "coordinates": [0] })
    );
    assert_eq!(
        variation(&semibold),
        serde_json::json!({ "axes": { "wght": 700.0 }, "coordinates": [12489] })
    );
    assert_eq!(variation(&clamped)["axes"]["wght"], 800.0);
    assert_eq!(
        bake_font(
            OXANIUM,
            BakeDescriptorV0::new(0).with_variation([("wdth".to_owned(), 100.0)]),
        )
        .expect_err("an axis the font lacks must fail")
        .code,
        BakeErrorCode::InvalidDescriptor,
    );
}

#[cfg(feature = "subsetting")]
#[test]
fn a_prepared_variable_subset_still_bakes_at_the_requested_instance() {
    use pmndrs_glyph_font_baker::{FontSelectionV0, UnicodeRangeV0, prepare_font};
    let prepared = prepare_font(
        OXANIUM,
        FontSelectionV0 {
            format_version: 0,
            font_face_index: 0,
            unicode_ranges: vec![UnicodeRangeV0 {
                start: 0x20,
                end: 0x7E,
            }],
        },
    )
    .expect("ASCII subset of a variable font");
    let baked = bake_font(
        &prepared.bytes,
        BakeDescriptorV0::new(0).with_variation([("wght".to_owned(), 700.0)]),
    )
    .expect("subset instance");
    assert!(
        baked
            .report
            .shared
            .shaping
            .tables
            .iter()
            .any(|table| table.tag == "HVAR")
    );
    assert!(prepared.report.glyph_count < 375);
}
