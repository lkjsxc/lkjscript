//! Unicode 17.0.0 default lowercase scalar mappings; context-free, including expansions.
//! Extracted from Rust's Unicode tables; Unicode Data Files and Software License:
//! https://www.unicode.org/license.txt
use crate::String;
pub fn append(value: char, result: &mut String) {
    let code = value as u32;
    if code < 128u32 { result.push(value.to_ascii_lowercase()); return; }
    if code == 304u32 {
        result.push('\u{69}');
        result.push('\u{307}');
        return;
    }
    if code >= 192u32 && code <= 214u32 && (code - 192u32) % 1u32 == 0u32 {
        result.push(char::from_u32(((code as i64) + (32i64)) as u32).unwrap()); return;
    }
    if code >= 216u32 && code <= 222u32 && (code - 216u32) % 1u32 == 0u32 {
        result.push(char::from_u32(((code as i64) + (32i64)) as u32).unwrap()); return;
    }
    if code >= 256u32 && code <= 302u32 && (code - 256u32) % 2u32 == 0u32 {
        result.push(char::from_u32(((code as i64) + (1i64)) as u32).unwrap()); return;
    }
    if code >= 306u32 && code <= 310u32 && (code - 306u32) % 2u32 == 0u32 {
        result.push(char::from_u32(((code as i64) + (1i64)) as u32).unwrap()); return;
    }
    if code >= 313u32 && code <= 327u32 && (code - 313u32) % 2u32 == 0u32 {
        result.push(char::from_u32(((code as i64) + (1i64)) as u32).unwrap()); return;
    }
    if code >= 330u32 && code <= 374u32 && (code - 330u32) % 2u32 == 0u32 {
        result.push(char::from_u32(((code as i64) + (1i64)) as u32).unwrap()); return;
    }
    if code == 376u32 { result.push(char::from_u32(255u32).unwrap()); return; }
    if code >= 377u32 && code <= 381u32 && (code - 377u32) % 2u32 == 0u32 {
        result.push(char::from_u32(((code as i64) + (1i64)) as u32).unwrap()); return;
    }
    if code == 385u32 { result.push(char::from_u32(595u32).unwrap()); return; }
    if code >= 386u32 && code <= 388u32 && (code - 386u32) % 2u32 == 0u32 {
        result.push(char::from_u32(((code as i64) + (1i64)) as u32).unwrap()); return;
    }
    if code == 390u32 { result.push(char::from_u32(596u32).unwrap()); return; }
    if code == 391u32 { result.push(char::from_u32(392u32).unwrap()); return; }
    if code >= 393u32 && code <= 394u32 && (code - 393u32) % 1u32 == 0u32 {
        result.push(char::from_u32(((code as i64) + (205i64)) as u32).unwrap()); return;
    }
    if code == 395u32 { result.push(char::from_u32(396u32).unwrap()); return; }
    if code == 398u32 { result.push(char::from_u32(477u32).unwrap()); return; }
    if code == 399u32 { result.push(char::from_u32(601u32).unwrap()); return; }
    if code == 400u32 { result.push(char::from_u32(603u32).unwrap()); return; }
    if code == 401u32 { result.push(char::from_u32(402u32).unwrap()); return; }
    if code == 403u32 { result.push(char::from_u32(608u32).unwrap()); return; }
    if code == 404u32 { result.push(char::from_u32(611u32).unwrap()); return; }
    if code == 406u32 { result.push(char::from_u32(617u32).unwrap()); return; }
    if code == 407u32 { result.push(char::from_u32(616u32).unwrap()); return; }
    if code == 408u32 { result.push(char::from_u32(409u32).unwrap()); return; }
    if code == 412u32 { result.push(char::from_u32(623u32).unwrap()); return; }
    if code == 413u32 { result.push(char::from_u32(626u32).unwrap()); return; }
    if code == 415u32 { result.push(char::from_u32(629u32).unwrap()); return; }
    if code >= 416u32 && code <= 420u32 && (code - 416u32) % 2u32 == 0u32 {
        result.push(char::from_u32(((code as i64) + (1i64)) as u32).unwrap()); return;
    }
    if code == 422u32 { result.push(char::from_u32(640u32).unwrap()); return; }
    if code == 423u32 { result.push(char::from_u32(424u32).unwrap()); return; }
    if code == 425u32 { result.push(char::from_u32(643u32).unwrap()); return; }
    if code == 428u32 { result.push(char::from_u32(429u32).unwrap()); return; }
    if code == 430u32 { result.push(char::from_u32(648u32).unwrap()); return; }
    if code == 431u32 { result.push(char::from_u32(432u32).unwrap()); return; }
    if code >= 433u32 && code <= 434u32 && (code - 433u32) % 1u32 == 0u32 {
        result.push(char::from_u32(((code as i64) + (217i64)) as u32).unwrap()); return;
    }
    if code >= 435u32 && code <= 437u32 && (code - 435u32) % 2u32 == 0u32 {
        result.push(char::from_u32(((code as i64) + (1i64)) as u32).unwrap()); return;
    }
    if code == 439u32 { result.push(char::from_u32(658u32).unwrap()); return; }
    if code == 440u32 { result.push(char::from_u32(441u32).unwrap()); return; }
    if code == 444u32 { result.push(char::from_u32(445u32).unwrap()); return; }
    if code == 452u32 { result.push(char::from_u32(454u32).unwrap()); return; }
    if code == 453u32 { result.push(char::from_u32(454u32).unwrap()); return; }
    if code == 455u32 { result.push(char::from_u32(457u32).unwrap()); return; }
    if code == 456u32 { result.push(char::from_u32(457u32).unwrap()); return; }
    if code == 458u32 { result.push(char::from_u32(460u32).unwrap()); return; }
    if code >= 459u32 && code <= 475u32 && (code - 459u32) % 2u32 == 0u32 {
        result.push(char::from_u32(((code as i64) + (1i64)) as u32).unwrap()); return;
    }
    if code >= 478u32 && code <= 494u32 && (code - 478u32) % 2u32 == 0u32 {
        result.push(char::from_u32(((code as i64) + (1i64)) as u32).unwrap()); return;
    }
    if code == 497u32 { result.push(char::from_u32(499u32).unwrap()); return; }
    if code >= 498u32 && code <= 500u32 && (code - 498u32) % 2u32 == 0u32 {
        result.push(char::from_u32(((code as i64) + (1i64)) as u32).unwrap()); return;
    }
    if code == 502u32 { result.push(char::from_u32(405u32).unwrap()); return; }
    if code == 503u32 { result.push(char::from_u32(447u32).unwrap()); return; }
    if code >= 504u32 && code <= 542u32 && (code - 504u32) % 2u32 == 0u32 {
        result.push(char::from_u32(((code as i64) + (1i64)) as u32).unwrap()); return;
    }
    if code == 544u32 { result.push(char::from_u32(414u32).unwrap()); return; }
    if code >= 546u32 && code <= 562u32 && (code - 546u32) % 2u32 == 0u32 {
        result.push(char::from_u32(((code as i64) + (1i64)) as u32).unwrap()); return;
    }
    if code == 570u32 { result.push(char::from_u32(11365u32).unwrap()); return; }
    if code == 571u32 { result.push(char::from_u32(572u32).unwrap()); return; }
    if code == 573u32 { result.push(char::from_u32(410u32).unwrap()); return; }
    if code == 574u32 { result.push(char::from_u32(11366u32).unwrap()); return; }
    if code == 577u32 { result.push(char::from_u32(578u32).unwrap()); return; }
    if code == 579u32 { result.push(char::from_u32(384u32).unwrap()); return; }
    if code == 580u32 { result.push(char::from_u32(649u32).unwrap()); return; }
    if code == 581u32 { result.push(char::from_u32(652u32).unwrap()); return; }
    if code >= 582u32 && code <= 590u32 && (code - 582u32) % 2u32 == 0u32 {
        result.push(char::from_u32(((code as i64) + (1i64)) as u32).unwrap()); return;
    }
    if code >= 880u32 && code <= 882u32 && (code - 880u32) % 2u32 == 0u32 {
        result.push(char::from_u32(((code as i64) + (1i64)) as u32).unwrap()); return;
    }
    if code == 886u32 { result.push(char::from_u32(887u32).unwrap()); return; }
    if code == 895u32 { result.push(char::from_u32(1011u32).unwrap()); return; }
    if code == 902u32 { result.push(char::from_u32(940u32).unwrap()); return; }
    if code >= 904u32 && code <= 906u32 && (code - 904u32) % 1u32 == 0u32 {
        result.push(char::from_u32(((code as i64) + (37i64)) as u32).unwrap()); return;
    }
    if code == 908u32 { result.push(char::from_u32(972u32).unwrap()); return; }
    if code >= 910u32 && code <= 911u32 && (code - 910u32) % 1u32 == 0u32 {
        result.push(char::from_u32(((code as i64) + (63i64)) as u32).unwrap()); return;
    }
    if code >= 913u32 && code <= 929u32 && (code - 913u32) % 1u32 == 0u32 {
        result.push(char::from_u32(((code as i64) + (32i64)) as u32).unwrap()); return;
    }
    if code >= 931u32 && code <= 939u32 && (code - 931u32) % 1u32 == 0u32 {
        result.push(char::from_u32(((code as i64) + (32i64)) as u32).unwrap()); return;
    }
    if code == 975u32 { result.push(char::from_u32(983u32).unwrap()); return; }
    if code >= 984u32 && code <= 1006u32 && (code - 984u32) % 2u32 == 0u32 {
        result.push(char::from_u32(((code as i64) + (1i64)) as u32).unwrap()); return;
    }
    if code == 1012u32 { result.push(char::from_u32(952u32).unwrap()); return; }
    if code == 1015u32 { result.push(char::from_u32(1016u32).unwrap()); return; }
    if code == 1017u32 { result.push(char::from_u32(1010u32).unwrap()); return; }
    if code == 1018u32 { result.push(char::from_u32(1019u32).unwrap()); return; }
    if code >= 1021u32 && code <= 1023u32 && (code - 1021u32) % 1u32 == 0u32 {
        result.push(char::from_u32(((code as i64) + (-130i64)) as u32).unwrap()); return;
    }
    if code >= 1024u32 && code <= 1039u32 && (code - 1024u32) % 1u32 == 0u32 {
        result.push(char::from_u32(((code as i64) + (80i64)) as u32).unwrap()); return;
    }
    if code >= 1040u32 && code <= 1071u32 && (code - 1040u32) % 1u32 == 0u32 {
        result.push(char::from_u32(((code as i64) + (32i64)) as u32).unwrap()); return;
    }
    if code >= 1120u32 && code <= 1152u32 && (code - 1120u32) % 2u32 == 0u32 {
        result.push(char::from_u32(((code as i64) + (1i64)) as u32).unwrap()); return;
    }
    if code >= 1162u32 && code <= 1214u32 && (code - 1162u32) % 2u32 == 0u32 {
        result.push(char::from_u32(((code as i64) + (1i64)) as u32).unwrap()); return;
    }
    if code == 1216u32 { result.push(char::from_u32(1231u32).unwrap()); return; }
    if code >= 1217u32 && code <= 1229u32 && (code - 1217u32) % 2u32 == 0u32 {
        result.push(char::from_u32(((code as i64) + (1i64)) as u32).unwrap()); return;
    }
    if code >= 1232u32 && code <= 1326u32 && (code - 1232u32) % 2u32 == 0u32 {
        result.push(char::from_u32(((code as i64) + (1i64)) as u32).unwrap()); return;
    }
    if code >= 1329u32 && code <= 1366u32 && (code - 1329u32) % 1u32 == 0u32 {
        result.push(char::from_u32(((code as i64) + (48i64)) as u32).unwrap()); return;
    }
    if code >= 4256u32 && code <= 4293u32 && (code - 4256u32) % 1u32 == 0u32 {
        result.push(char::from_u32(((code as i64) + (7264i64)) as u32).unwrap()); return;
    }
    if code == 4295u32 { result.push(char::from_u32(11559u32).unwrap()); return; }
    if code == 4301u32 { result.push(char::from_u32(11565u32).unwrap()); return; }
    if code >= 5024u32 && code <= 5103u32 && (code - 5024u32) % 1u32 == 0u32 {
        result.push(char::from_u32(((code as i64) + (38864i64)) as u32).unwrap()); return;
    }
    if code >= 5104u32 && code <= 5109u32 && (code - 5104u32) % 1u32 == 0u32 {
        result.push(char::from_u32(((code as i64) + (8i64)) as u32).unwrap()); return;
    }
    if code == 7305u32 { result.push(char::from_u32(7306u32).unwrap()); return; }
    if code >= 7312u32 && code <= 7354u32 && (code - 7312u32) % 1u32 == 0u32 {
        result.push(char::from_u32(((code as i64) + (-3008i64)) as u32).unwrap()); return;
    }
    if code >= 7357u32 && code <= 7359u32 && (code - 7357u32) % 1u32 == 0u32 {
        result.push(char::from_u32(((code as i64) + (-3008i64)) as u32).unwrap()); return;
    }
    if code >= 7680u32 && code <= 7828u32 && (code - 7680u32) % 2u32 == 0u32 {
        result.push(char::from_u32(((code as i64) + (1i64)) as u32).unwrap()); return;
    }
    if code == 7838u32 { result.push(char::from_u32(223u32).unwrap()); return; }
    if code >= 7840u32 && code <= 7934u32 && (code - 7840u32) % 2u32 == 0u32 {
        result.push(char::from_u32(((code as i64) + (1i64)) as u32).unwrap()); return;
    }
    if code >= 7944u32 && code <= 7951u32 && (code - 7944u32) % 1u32 == 0u32 {
        result.push(char::from_u32(((code as i64) + (-8i64)) as u32).unwrap()); return;
    }
    if code >= 7960u32 && code <= 7965u32 && (code - 7960u32) % 1u32 == 0u32 {
        result.push(char::from_u32(((code as i64) + (-8i64)) as u32).unwrap()); return;
    }
    if code >= 7976u32 && code <= 7983u32 && (code - 7976u32) % 1u32 == 0u32 {
        result.push(char::from_u32(((code as i64) + (-8i64)) as u32).unwrap()); return;
    }
    if code >= 7992u32 && code <= 7999u32 && (code - 7992u32) % 1u32 == 0u32 {
        result.push(char::from_u32(((code as i64) + (-8i64)) as u32).unwrap()); return;
    }
    if code >= 8008u32 && code <= 8013u32 && (code - 8008u32) % 1u32 == 0u32 {
        result.push(char::from_u32(((code as i64) + (-8i64)) as u32).unwrap()); return;
    }
    if code >= 8025u32 && code <= 8031u32 && (code - 8025u32) % 2u32 == 0u32 {
        result.push(char::from_u32(((code as i64) + (-8i64)) as u32).unwrap()); return;
    }
    if code >= 8040u32 && code <= 8047u32 && (code - 8040u32) % 1u32 == 0u32 {
        result.push(char::from_u32(((code as i64) + (-8i64)) as u32).unwrap()); return;
    }
    if code >= 8072u32 && code <= 8079u32 && (code - 8072u32) % 1u32 == 0u32 {
        result.push(char::from_u32(((code as i64) + (-8i64)) as u32).unwrap()); return;
    }
    if code >= 8088u32 && code <= 8095u32 && (code - 8088u32) % 1u32 == 0u32 {
        result.push(char::from_u32(((code as i64) + (-8i64)) as u32).unwrap()); return;
    }
    if code >= 8104u32 && code <= 8111u32 && (code - 8104u32) % 1u32 == 0u32 {
        result.push(char::from_u32(((code as i64) + (-8i64)) as u32).unwrap()); return;
    }
    if code >= 8120u32 && code <= 8121u32 && (code - 8120u32) % 1u32 == 0u32 {
        result.push(char::from_u32(((code as i64) + (-8i64)) as u32).unwrap()); return;
    }
    if code >= 8122u32 && code <= 8123u32 && (code - 8122u32) % 1u32 == 0u32 {
        result.push(char::from_u32(((code as i64) + (-74i64)) as u32).unwrap()); return;
    }
    if code == 8124u32 { result.push(char::from_u32(8115u32).unwrap()); return; }
    if code >= 8136u32 && code <= 8139u32 && (code - 8136u32) % 1u32 == 0u32 {
        result.push(char::from_u32(((code as i64) + (-86i64)) as u32).unwrap()); return;
    }
    if code == 8140u32 { result.push(char::from_u32(8131u32).unwrap()); return; }
    if code >= 8152u32 && code <= 8153u32 && (code - 8152u32) % 1u32 == 0u32 {
        result.push(char::from_u32(((code as i64) + (-8i64)) as u32).unwrap()); return;
    }
    if code >= 8154u32 && code <= 8155u32 && (code - 8154u32) % 1u32 == 0u32 {
        result.push(char::from_u32(((code as i64) + (-100i64)) as u32).unwrap()); return;
    }
    if code >= 8168u32 && code <= 8169u32 && (code - 8168u32) % 1u32 == 0u32 {
        result.push(char::from_u32(((code as i64) + (-8i64)) as u32).unwrap()); return;
    }
    if code >= 8170u32 && code <= 8171u32 && (code - 8170u32) % 1u32 == 0u32 {
        result.push(char::from_u32(((code as i64) + (-112i64)) as u32).unwrap()); return;
    }
    if code == 8172u32 { result.push(char::from_u32(8165u32).unwrap()); return; }
    if code >= 8184u32 && code <= 8185u32 && (code - 8184u32) % 1u32 == 0u32 {
        result.push(char::from_u32(((code as i64) + (-128i64)) as u32).unwrap()); return;
    }
    if code >= 8186u32 && code <= 8187u32 && (code - 8186u32) % 1u32 == 0u32 {
        result.push(char::from_u32(((code as i64) + (-126i64)) as u32).unwrap()); return;
    }
    if code == 8188u32 { result.push(char::from_u32(8179u32).unwrap()); return; }
    if code == 8486u32 { result.push(char::from_u32(969u32).unwrap()); return; }
    if code == 8490u32 { result.push(char::from_u32(107u32).unwrap()); return; }
    if code == 8491u32 { result.push(char::from_u32(229u32).unwrap()); return; }
    if code == 8498u32 { result.push(char::from_u32(8526u32).unwrap()); return; }
    if code >= 8544u32 && code <= 8559u32 && (code - 8544u32) % 1u32 == 0u32 {
        result.push(char::from_u32(((code as i64) + (16i64)) as u32).unwrap()); return;
    }
    if code == 8579u32 { result.push(char::from_u32(8580u32).unwrap()); return; }
    if code >= 9398u32 && code <= 9423u32 && (code - 9398u32) % 1u32 == 0u32 {
        result.push(char::from_u32(((code as i64) + (26i64)) as u32).unwrap()); return;
    }
    if code >= 11264u32 && code <= 11311u32 && (code - 11264u32) % 1u32 == 0u32 {
        result.push(char::from_u32(((code as i64) + (48i64)) as u32).unwrap()); return;
    }
    if code == 11360u32 { result.push(char::from_u32(11361u32).unwrap()); return; }
    if code == 11362u32 { result.push(char::from_u32(619u32).unwrap()); return; }
    if code == 11363u32 { result.push(char::from_u32(7549u32).unwrap()); return; }
    if code == 11364u32 { result.push(char::from_u32(637u32).unwrap()); return; }
    if code >= 11367u32 && code <= 11371u32 && (code - 11367u32) % 2u32 == 0u32 {
        result.push(char::from_u32(((code as i64) + (1i64)) as u32).unwrap()); return;
    }
    if code == 11373u32 { result.push(char::from_u32(593u32).unwrap()); return; }
    if code == 11374u32 { result.push(char::from_u32(625u32).unwrap()); return; }
    if code == 11375u32 { result.push(char::from_u32(592u32).unwrap()); return; }
    if code == 11376u32 { result.push(char::from_u32(594u32).unwrap()); return; }
    if code == 11378u32 { result.push(char::from_u32(11379u32).unwrap()); return; }
    if code == 11381u32 { result.push(char::from_u32(11382u32).unwrap()); return; }
    if code >= 11390u32 && code <= 11391u32 && (code - 11390u32) % 1u32 == 0u32 {
        result.push(char::from_u32(((code as i64) + (-10815i64)) as u32).unwrap()); return;
    }
    if code >= 11392u32 && code <= 11490u32 && (code - 11392u32) % 2u32 == 0u32 {
        result.push(char::from_u32(((code as i64) + (1i64)) as u32).unwrap()); return;
    }
    if code >= 11499u32 && code <= 11501u32 && (code - 11499u32) % 2u32 == 0u32 {
        result.push(char::from_u32(((code as i64) + (1i64)) as u32).unwrap()); return;
    }
    if code == 11506u32 { result.push(char::from_u32(11507u32).unwrap()); return; }
    if code >= 42560u32 && code <= 42604u32 && (code - 42560u32) % 2u32 == 0u32 {
        result.push(char::from_u32(((code as i64) + (1i64)) as u32).unwrap()); return;
    }
    if code >= 42624u32 && code <= 42650u32 && (code - 42624u32) % 2u32 == 0u32 {
        result.push(char::from_u32(((code as i64) + (1i64)) as u32).unwrap()); return;
    }
    if code >= 42786u32 && code <= 42798u32 && (code - 42786u32) % 2u32 == 0u32 {
        result.push(char::from_u32(((code as i64) + (1i64)) as u32).unwrap()); return;
    }
    if code >= 42802u32 && code <= 42862u32 && (code - 42802u32) % 2u32 == 0u32 {
        result.push(char::from_u32(((code as i64) + (1i64)) as u32).unwrap()); return;
    }
    if code >= 42873u32 && code <= 42875u32 && (code - 42873u32) % 2u32 == 0u32 {
        result.push(char::from_u32(((code as i64) + (1i64)) as u32).unwrap()); return;
    }
    if code == 42877u32 { result.push(char::from_u32(7545u32).unwrap()); return; }
    if code >= 42878u32 && code <= 42886u32 && (code - 42878u32) % 2u32 == 0u32 {
        result.push(char::from_u32(((code as i64) + (1i64)) as u32).unwrap()); return;
    }
    if code == 42891u32 { result.push(char::from_u32(42892u32).unwrap()); return; }
    if code == 42893u32 { result.push(char::from_u32(613u32).unwrap()); return; }
    if code >= 42896u32 && code <= 42898u32 && (code - 42896u32) % 2u32 == 0u32 {
        result.push(char::from_u32(((code as i64) + (1i64)) as u32).unwrap()); return;
    }
    if code >= 42902u32 && code <= 42920u32 && (code - 42902u32) % 2u32 == 0u32 {
        result.push(char::from_u32(((code as i64) + (1i64)) as u32).unwrap()); return;
    }
    if code == 42922u32 { result.push(char::from_u32(614u32).unwrap()); return; }
    if code == 42923u32 { result.push(char::from_u32(604u32).unwrap()); return; }
    if code == 42924u32 { result.push(char::from_u32(609u32).unwrap()); return; }
    if code == 42925u32 { result.push(char::from_u32(620u32).unwrap()); return; }
    if code == 42926u32 { result.push(char::from_u32(618u32).unwrap()); return; }
    if code == 42928u32 { result.push(char::from_u32(670u32).unwrap()); return; }
    if code == 42929u32 { result.push(char::from_u32(647u32).unwrap()); return; }
    if code == 42930u32 { result.push(char::from_u32(669u32).unwrap()); return; }
    if code == 42931u32 { result.push(char::from_u32(43859u32).unwrap()); return; }
    if code >= 42932u32 && code <= 42946u32 && (code - 42932u32) % 2u32 == 0u32 {
        result.push(char::from_u32(((code as i64) + (1i64)) as u32).unwrap()); return;
    }
    if code == 42948u32 { result.push(char::from_u32(42900u32).unwrap()); return; }
    if code == 42949u32 { result.push(char::from_u32(642u32).unwrap()); return; }
    if code == 42950u32 { result.push(char::from_u32(7566u32).unwrap()); return; }
    if code >= 42951u32 && code <= 42953u32 && (code - 42951u32) % 2u32 == 0u32 {
        result.push(char::from_u32(((code as i64) + (1i64)) as u32).unwrap()); return;
    }
    if code == 42955u32 { result.push(char::from_u32(612u32).unwrap()); return; }
    if code >= 42956u32 && code <= 42970u32 && (code - 42956u32) % 2u32 == 0u32 {
        result.push(char::from_u32(((code as i64) + (1i64)) as u32).unwrap()); return;
    }
    if code == 42972u32 { result.push(char::from_u32(411u32).unwrap()); return; }
    if code == 42997u32 { result.push(char::from_u32(42998u32).unwrap()); return; }
    if code >= 65313u32 && code <= 65338u32 && (code - 65313u32) % 1u32 == 0u32 {
        result.push(char::from_u32(((code as i64) + (32i64)) as u32).unwrap()); return;
    }
    if code >= 66560u32 && code <= 66599u32 && (code - 66560u32) % 1u32 == 0u32 {
        result.push(char::from_u32(((code as i64) + (40i64)) as u32).unwrap()); return;
    }
    if code >= 66736u32 && code <= 66771u32 && (code - 66736u32) % 1u32 == 0u32 {
        result.push(char::from_u32(((code as i64) + (40i64)) as u32).unwrap()); return;
    }
    if code >= 66928u32 && code <= 66938u32 && (code - 66928u32) % 1u32 == 0u32 {
        result.push(char::from_u32(((code as i64) + (39i64)) as u32).unwrap()); return;
    }
    if code >= 66940u32 && code <= 66954u32 && (code - 66940u32) % 1u32 == 0u32 {
        result.push(char::from_u32(((code as i64) + (39i64)) as u32).unwrap()); return;
    }
    if code >= 66956u32 && code <= 66962u32 && (code - 66956u32) % 1u32 == 0u32 {
        result.push(char::from_u32(((code as i64) + (39i64)) as u32).unwrap()); return;
    }
    if code >= 66964u32 && code <= 66965u32 && (code - 66964u32) % 1u32 == 0u32 {
        result.push(char::from_u32(((code as i64) + (39i64)) as u32).unwrap()); return;
    }
    if code >= 68736u32 && code <= 68786u32 && (code - 68736u32) % 1u32 == 0u32 {
        result.push(char::from_u32(((code as i64) + (64i64)) as u32).unwrap()); return;
    }
    if code >= 68944u32 && code <= 68965u32 && (code - 68944u32) % 1u32 == 0u32 {
        result.push(char::from_u32(((code as i64) + (32i64)) as u32).unwrap()); return;
    }
    if code >= 71840u32 && code <= 71871u32 && (code - 71840u32) % 1u32 == 0u32 {
        result.push(char::from_u32(((code as i64) + (32i64)) as u32).unwrap()); return;
    }
    if code >= 93760u32 && code <= 93791u32 && (code - 93760u32) % 1u32 == 0u32 {
        result.push(char::from_u32(((code as i64) + (32i64)) as u32).unwrap()); return;
    }
    if code >= 93856u32 && code <= 93880u32 && (code - 93856u32) % 1u32 == 0u32 {
        result.push(char::from_u32(((code as i64) + (27i64)) as u32).unwrap()); return;
    }
    if code >= 125184u32 && code <= 125217u32 && (code - 125184u32) % 1u32 == 0u32 {
        result.push(char::from_u32(((code as i64) + (34i64)) as u32).unwrap()); return;
    }
    result.push(value);
}
