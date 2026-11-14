//! Convert a local original-game battle record into an engine fixture and run it.
//!
//! Inputs:
//! - original client `.bin` records (`RecentBattleInfo` protobuf wire format)
//! - decoded JSON records (`roundStats`, with either the client field names or the
//!   third-party mirror field names)
//! - JSONL files containing either of the above
//!
//! The converter only uses public engine data: card definitions are resolved
//! through the engine catalog and level HP through `shared/data/level-config.json`.
//! It never uploads or bundles replay data.

use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use serde_json::{json, Map, Value};
use yixian_engine::{
    original_card_definition_by_id, run_replay_fixture, BattleFixture, CardDefinition,
};

const LEVEL_CONFIG_JSON: &str = include_str!("../../../shared/data/level-config.json");
const DECK_SLOTS: usize = 8;

fn main() {
    if let Err(error) = run() {
        eprintln!("Error: {error}");
        std::process::exit(1);
    }
}

struct Cli {
    input: PathBuf,
    round: Option<i64>,
    write_fixtures: Option<PathBuf>,
    help: bool,
}

fn run() -> Result<(), String> {
    let cli = parse_cli(env::args().skip(1))?;
    if cli.help {
        print_usage();
        return Ok(());
    }

    let bytes = fs::read(&cli.input)
        .map_err(|error| format!("cannot read {}: {error}", cli.input.display()))?;
    let records = decode_input(&bytes, &cli.input)?;
    if records.is_empty() {
        return Err(format!("no records in {}", cli.input.display()));
    }

    let stem = file_stem(&cli.input);
    let mut rows = Vec::new();
    let mut failed = false;

    for (record_index, record) in records.iter().enumerate() {
        let battles = record
            .get("roundStats")
            .and_then(Value::as_array)
            .ok_or_else(|| format!("record {} has no roundStats", record_index + 1))?;
        let mut matched = 0usize;
        for battle in battles {
            let round = int_or(battle.get("round"), 0);
            if let Some(wanted) = cli.round {
                if round != wanted {
                    continue;
                }
            }
            matched += 1;

            let fixture_value = match build_fixture(record, battle) {
                Ok(value) => value,
                Err(error) => {
                    failed = true;
                    rows.push(json!({
                        "record": record_index + 1,
                        "round": round,
                        "error": error,
                    }));
                    continue;
                }
            };

            let fixture_path = if let Some(directory) = &cli.write_fixtures {
                fs::create_dir_all(directory)
                    .map_err(|error| format!("cannot create {}: {error}", directory.display()))?;
                let name = fixture_file_name(&stem, record_index, records.len(), round);
                let path = directory.join(name);
                let text = serde_json::to_string_pretty(&fixture_value)
                    .map_err(|error| format!("fixture serialization failed: {error}"))?;
                fs::write(&path, format!("{text}\n"))
                    .map_err(|error| format!("cannot write {}: {error}", path.display()))?;
                Some(path)
            } else {
                None
            };

            let fixture: BattleFixture = match serde_json::from_value(fixture_value) {
                Ok(value) => value,
                Err(error) => {
                    failed = true;
                    rows.push(json!({
                        "record": record_index + 1,
                        "round": round,
                        "fixturePath": fixture_path.map(|path| path.display().to_string()),
                        "error": format!("fixture contract rejected: {error}"),
                    }));
                    continue;
                }
            };

            match run_replay_fixture(&fixture) {
                Ok(summary) => {
                    let exact = summary.matches_fixture(&fixture);
                    if !exact {
                        failed = true;
                    }
                    rows.push(json!({
                        "record": record_index + 1,
                        "round": round,
                        "fixturePath": fixture_path.map(|path| path.display().to_string()),
                        "expected": {
                            "winnerSide": fixture.expected.winner_side,
                            "actorTurnCount": fixture.expected.actor_turn_count,
                            "hpDeltaP1MinusP2": fixture.expected.hp_delta_p1_minus_p2,
                        },
                        "actual": {
                            "winnerSide": summary.winner_side,
                            "actorTurnCount": summary.actor_turn_count,
                            "hpDeltaP1MinusP2": summary.hp_delta_p1_minus_p2,
                        },
                        "exact": exact,
                    }));
                }
                Err(error) => {
                    failed = true;
                    rows.push(json!({
                        "record": record_index + 1,
                        "round": round,
                        "fixturePath": fixture_path.map(|path| path.display().to_string()),
                        "error": error.to_string(),
                    }));
                }
            }
        }
        if matched == 0 && cli.round.is_some() {
            rows.push(json!({
                "record": record_index + 1,
                "round": cli.round,
                "error": "round not found",
            }));
            failed = true;
        }
    }

    let report = json!({
        "input": cli.input.display().to_string(),
        "rounds": rows,
    });
    println!(
        "{}",
        serde_json::to_string_pretty(&report)
            .map_err(|error| format!("report serialization failed: {error}"))?
    );
    if failed {
        return Err("one or more rounds did not replay exact".to_string());
    }
    Ok(())
}

fn fixture_file_name(stem: &str, record_index: usize, record_count: usize, round: i64) -> String {
    if record_count > 1 {
        format!("{stem}-r{:02}-round-{round:02}.json", record_index + 1)
    } else {
        format!("{stem}-round-{round:02}.json")
    }
}

fn file_stem(path: &Path) -> String {
    path.file_stem()
        .and_then(|stem| stem.to_str())
        .filter(|stem| !stem.is_empty())
        .unwrap_or("replay")
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                ch
            } else {
                '_'
            }
        })
        .collect()
}

fn parse_cli<I>(args: I) -> Result<Cli, String>
where
    I: IntoIterator<Item = String>,
{
    let mut input = None;
    let mut round = None;
    let mut write_fixtures = None;
    let mut help = false;
    let mut args = args.into_iter();

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-h" | "--help" => help = true,
            "--round" => {
                let value = args.next().ok_or("--round needs a value")?;
                round = Some(
                    value
                        .parse::<i64>()
                        .map_err(|_| format!("invalid round: {value}"))?,
                );
            }
            "--write-fixtures" => {
                write_fixtures = Some(PathBuf::from(
                    args.next().ok_or("--write-fixtures needs a path")?,
                ));
            }
            value if value.starts_with('-') => return Err(format!("unknown option: {value}")),
            value => {
                if input.is_some() {
                    return Err(format!("unexpected argument: {value}"));
                }
                input = Some(PathBuf::from(value));
            }
        }
    }

    if !help {
        if input.is_none() {
            return Err("missing input path".to_string());
        }
    }

    Ok(Cli {
        input: input.unwrap_or_default(),
        round,
        write_fixtures,
        help,
    })
}

fn print_usage() {
    println!(
        "Convert a local original-game record to an engine fixture and run it.\n\
\n\
Usage:\n\
  replay_import <input> [--round N] [--write-fixtures DIR]\n\
\n\
Input:\n\
  .bin                      original client RecentBattleInfo record\n\
  .json / .jsonl            decoded record, mirror record, or JSONL of either\n\
\n\
Options:\n\
  --round N                 import only round N (default: every round)\n\
  --write-fixtures DIR      also write each fixture JSON under DIR\n\
  -h, --help                show this help"
    );
}

fn decode_input(bytes: &[u8], path: &Path) -> Result<Vec<Value>, String> {
    let is_bin = path
        .extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| extension.eq_ignore_ascii_case("bin"))
        .unwrap_or(false);
    if is_bin || !looks_like_json(bytes) {
        return decode_recent_battle_bin(bytes).map(|record| vec![record]);
    }

    let text = std::str::from_utf8(bytes)
        .map_err(|error| format!("input is not valid UTF-8 JSON: {error}"))?;
    if let Ok(value) = serde_json::from_str::<Value>(text) {
        return normalize_json_records(value);
    }

    let mut records = Vec::new();
    for (line_index, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let value = serde_json::from_str::<Value>(line)
            .map_err(|error| format!("JSONL line {} is invalid: {error}", line_index + 1))?;
        records.extend(normalize_json_records(value)?);
    }
    Ok(records)
}

fn looks_like_json(bytes: &[u8]) -> bool {
    bytes
        .iter()
        .find(|byte| !byte.is_ascii_whitespace())
        .map(|byte| *byte == b'{' || *byte == b'[')
        .unwrap_or(false)
}

fn normalize_json_records(value: Value) -> Result<Vec<Value>, String> {
    match value {
        Value::Array(items) => {
            let mut records = Vec::new();
            for item in items {
                records.extend(normalize_json_records(item)?);
            }
            Ok(records)
        }
        Value::Object(mut object) => {
            if let Some(data) = object.remove("data") {
                return normalize_json_records(data);
            }
            if let Some(record) = object.remove("record") {
                return normalize_json_records(record);
            }
            if !object.contains_key("roundStats") {
                return Err("JSON record has no roundStats".to_string());
            }
            Ok(vec![adapt_record(Value::Object(object))?])
        }
        _ => Err("JSON input must be an object or array".to_string()),
    }
}

fn adapt_record(record: Value) -> Result<Value, String> {
    let mut object = record
        .as_object()
        .cloned()
        .ok_or("record must be an object")?;
    let battles = object
        .get("roundStats")
        .and_then(Value::as_array)
        .cloned()
        .ok_or("record has no roundStats array")?;
    let record_season =
        int_value(object.get("seasonMec")).or_else(|| int_value(object.get("seasonMechanism")));
    let mut adapted = Vec::with_capacity(battles.len());
    for battle in &battles {
        adapted.push(adapt_battle(battle, record_season)?);
    }
    object.insert("roundStats".to_string(), Value::Array(adapted));
    object.insert(
        "seasonMechanism".to_string(),
        json!(record_season.unwrap_or(0)),
    );
    if let Some(season) = record_season {
        object.insert("seasonMec".to_string(), json!(season));
    }
    Ok(Value::Object(object))
}

fn adapt_battle(battle: &Value, record_season: Option<i64>) -> Result<Value, String> {
    let mut object = battle
        .as_object()
        .cloned()
        .ok_or("roundStats entry must be an object")?;
    let is_mirror = object.contains_key("winerId")
        || object.contains_key("huiHeCount")
        || object.contains_key("publicData")
        || object.contains_key("privateData")
        || object.contains_key("lastRoundData")
        || object.contains_key("seasonMec");
    if !is_mirror {
        return Ok(Value::Object(object));
    }

    let winner = object
        .get("winnerId")
        .or_else(|| object.get("winerId"))
        .cloned();
    if let Some(winner) = winner {
        object.insert("winnerId".to_string(), winner);
    }
    let actor_turns = object
        .get("actorTurnCount")
        .or_else(|| object.get("huiHeCount"))
        .cloned();
    if let Some(actor_turns) = actor_turns {
        object.insert("actorTurnCount".to_string(), actor_turns);
    }
    let season = int_value(object.get("seasonMec"))
        .or_else(|| int_value(object.get("seasonMechanism")))
        .or(record_season)
        .unwrap_or(0);
    object.insert("seasonMechanism".to_string(), json!(season));

    for side in ["p1", "p2"] {
        let player = object
            .get(side)
            .cloned()
            .ok_or_else(|| format!("round is missing {side}"))?;
        object.insert(side.to_string(), adapt_player(&player)?);
    }
    Ok(Value::Object(object))
}

fn adapt_player(player: &Value) -> Result<Value, String> {
    let object = player.as_object().ok_or("player must be an object")?;
    let public = object
        .get("publicData")
        .or_else(|| object.get("public"))
        .cloned()
        .unwrap_or_else(|| json!({}));
    let private = object
        .get("privateData")
        .or_else(|| object.get("private"))
        .cloned()
        .unwrap_or_else(|| json!({}));
    let mut public = public
        .as_object()
        .cloned()
        .ok_or("player.public must be an object")?;
    let mut private = private
        .as_object()
        .cloned()
        .ok_or("player.private must be an object")?;

    let last_round = public
        .remove("lastRoundData")
        .or_else(|| public.remove("lastRound"))
        .unwrap_or_else(|| json!({}));
    let mut last_round = last_round
        .as_object()
        .cloned()
        .ok_or("player.lastRound must be an object")?;

    normalize_talent_map(&mut public);
    normalize_talent_map(&mut private);
    normalize_talent_map(&mut last_round);

    let last_used = int_list(last_round.get("usedCards"));
    let used = if last_used.is_empty() {
        int_list(private.get("usedCards"))
    } else {
        last_used
    };
    let mut used = used;
    if used.len() > DECK_SLOTS {
        return Err(format!(
            "usedCards has {} slots, expected at most {DECK_SLOTS}",
            used.len()
        ));
    }
    used.resize(DECK_SLOTS, 0);
    private.insert("usedCards".to_string(), json!(used));

    if let Some(hand) = last_round
        .get("handCards")
        .and_then(Value::as_array)
        .cloned()
    {
        let hand: Vec<Value> = hand.into_iter().filter(|item| !item.is_null()).collect();
        last_round.insert("handCards".to_string(), Value::Array(hand));
    }

    public.insert("lastRound".to_string(), Value::Object(last_round));
    Ok(json!({
        "public": Value::Object(public),
        "private": Value::Object(private),
    }))
}

fn normalize_talent_map(holder: &mut Map<String, Value>) {
    let Some(Value::Object(map)) = holder.get("talentDatas") else {
        return;
    };
    let mut list = Vec::with_capacity(map.len());
    for (key, value) in map {
        let Ok(id) = key.parse::<i64>() else {
            continue;
        };
        let mut item = value.as_object().cloned().unwrap_or_default();
        item.insert("id".to_string(), json!(id));
        list.push(Value::Object(item));
    }
    holder.insert("talentDatas".to_string(), Value::Array(list));
}

fn build_fixture(record: &Value, battle: &Value) -> Result<Value, String> {
    let p1 = battle.get("p1").ok_or("round is missing p1")?;
    let p2 = battle.get("p2").ok_or("round is missing p2")?;
    let p1_public = p1.get("public").ok_or("p1 is missing public state")?;
    let p2_public = p2.get("public").ok_or("p2 is missing public state")?;
    let p1_uid = string_value(p1_public.get("uid"));
    let p2_uid = string_value(p2_public.get("uid"));
    let first_player_id = string_value(battle.get("firstPlayerId"));
    let winner_id = string_value(battle.get("winnerId"));
    let first_player_side = side_for_uid(&first_player_id, &p1_uid, &p2_uid, "first player")?;
    let winner_side = side_for_uid(&winner_id, &p1_uid, &p2_uid, "winner")?;
    let round = int_or(battle.get("round"), 0);
    let actor_turn_count = int_value(battle.get("actorTurnCount"))
        .ok_or("round is missing actorTurnCount (huiHeCount)")?;
    let hp_delta = int_or(battle.get("hpDelta"), 0);
    let season_mechanism = int_or(battle.get("seasonMechanism"), 0);

    let mut source = Map::new();
    source.insert("round".to_string(), json!(round));
    source.insert("seasonMechanism".to_string(), json!(season_mechanism));
    if let Some(version) = record
        .get("gameVersion")
        .or_else(|| record.get("version"))
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
    {
        source.insert("gameVersion".to_string(), json!(version));
    }
    if let Some(build) = record
        .get("steamBuild")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
    {
        source.insert("steamBuild".to_string(), json!(build));
    }

    Ok(json!({
        "schemaVersion": 1,
        "source": Value::Object(source),
        "firstPlayerSide": first_player_side,
        "decisionTape": int_list(battle.get("battleParams")),
        "expected": {
            "winnerSide": winner_side,
            "actorTurnCount": actor_turn_count,
            "hpDeltaP1MinusP2": hp_delta,
        },
        "players": {
            "p1": build_player(p1)?,
            "p2": build_player(p2)?,
        },
    }))
}

fn build_player(player: &Value) -> Result<Value, String> {
    let public = player
        .get("public")
        .and_then(Value::as_object)
        .ok_or("player.public must be an object")?;
    let private = player
        .get("private")
        .and_then(Value::as_object)
        .ok_or("player.private must be an object")?;
    let empty = Map::new();
    let last_round = public
        .get("lastRound")
        .and_then(Value::as_object)
        .unwrap_or(&empty);

    let level = int_value(last_round.get("level")).ok_or("player.lastRound.level is missing")?;
    let base_max_hp = base_max_hp_for_level(level)?;
    let mut card_ids = int_list(private.get("usedCards"));
    if card_ids.len() > DECK_SLOTS {
        return Err(format!(
            "usedCards has {} slots, expected at most {DECK_SLOTS}",
            card_ids.len()
        ));
    }
    card_ids.resize(DECK_SLOTS, 0);
    let last_used = int_list(last_round.get("usedCards"));
    let base_ids: Vec<i64> = if last_used.is_empty() {
        card_ids.iter().copied().collect()
    } else {
        last_used.iter().copied().collect()
    };

    let mut cards = Vec::with_capacity(card_ids.len());
    for card_id in &card_ids {
        let definition: CardDefinition = original_card_definition_by_id(*card_id)
            .ok_or_else(|| format!("current rule snapshot is missing card {card_id}"))?;
        cards.push(
            serde_json::to_value(&definition)
                .map_err(|error| format!("card {card_id} serialization failed: {error}"))?,
        );
    }

    let active_slot_count = int_value(private.get("unlockGrids"))
        .unwrap_or(card_ids.len() as i64)
        .clamp(1, DECK_SLOTS as i64);
    let talent_card_params = talent_card_params(public, private);
    let xian_mo_strategies = {
        let from_round = int_list(last_round.get("xianMoStrategies"));
        if from_round.is_empty() {
            xian_mo_selected(private)
        } else {
            from_round
        }
    };
    let xian_mo_temp_datas = private
        .get("xianMoData")
        .and_then(|data| data.get("tempDatas"))
        .map(|value| number_map(Some(value)))
        .unwrap_or_default();
    let fate_strategy_temp_datas = private
        .get("fateStrategyData")
        .and_then(|data| data.get("tempDatas"))
        .map(|value| number_map(Some(value)))
        .unwrap_or_default();

    let mut value = json!({
        "level": level,
        "baseMaxHp": base_max_hp,
        "extraMaxHp": int_or(last_round.get("extraMaxHp"), 0),
        "characterId": int_or(public.get("characterId"), 0),
        "talents": int_list(last_round.get("talents")),
        "activeSlotCount": active_slot_count,
        "handCards": int_list(last_round.get("handCards")),
        "lastRoundLife": int_or(last_round.get("life"), 0),
        "lastRoundExp": int_or(last_round.get("exp"), 0),
        "lastRoundUsedCardBaseIds": base_ids
            .iter()
            .map(|card_id| normalize_base_id(*card_id))
            .collect::<Vec<_>>(),
        "talentTempDatas": number_map(last_round.get("talentTempDatas")),
        "permanentBuffTempDatas": number_map(last_round.get("permanentBuffTempDatas")),
        "cards": cards,
    });
    let object = value.as_object_mut().expect("fixture player object");

    let used_ke_yin = int_list(last_round.get("usedKeYinCards"));
    if !used_ke_yin.is_empty() {
        object.insert("usedKeYinCards".to_string(), json!(used_ke_yin));
    }
    let fate_strategies = int_list(last_round.get("fateStrategies"));
    if !fate_strategies.is_empty() {
        object.insert("fateStrategies".to_string(), json!(fate_strategies));
    }
    if !xian_mo_strategies.is_empty() {
        object.insert("xianMoStrategies".to_string(), json!(xian_mo_strategies));
    }
    if !xian_mo_temp_datas.is_empty() {
        object.insert("xianMoTempDatas".to_string(), json!(xian_mo_temp_datas));
    }
    if !talent_card_params.is_empty() {
        object.insert("talentCardParams".to_string(), json!(talent_card_params));
    }
    if !fate_strategy_temp_datas.is_empty() {
        object.insert(
            "fateStrategyTempDatas".to_string(),
            json!(fate_strategy_temp_datas),
        );
    }
    Ok(value)
}

fn talent_card_params(
    public: &Map<String, Value>,
    private: &Map<String, Value>,
) -> BTreeMap<String, Vec<i64>> {
    let mut result = BTreeMap::new();
    for holder in [public, private] {
        let Some(items) = holder.get("talentDatas").and_then(Value::as_array) else {
            continue;
        };
        for item in items {
            let Some(id) = int_value(item.get("id")) else {
                continue;
            };
            let params = int_list(item.get("commonParams"));
            if !params.is_empty() {
                result.insert(id.to_string(), params);
            }
        }
    }
    result
}

fn xian_mo_selected(private: &Map<String, Value>) -> Vec<i64> {
    private
        .get("xianMoData")
        .and_then(|data| data.get("strategies"))
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| int_value(item.get("selected")))
                .collect()
        })
        .unwrap_or_default()
}

fn base_max_hp_for_level(level: i64) -> Result<i64, String> {
    static LEVELS: OnceLock<BTreeMap<i64, i64>> = OnceLock::new();
    let levels = LEVELS.get_or_init(|| {
        let value: Value =
            serde_json::from_str(LEVEL_CONFIG_JSON).expect("shared/data/level-config.json parses");
        let mut map = BTreeMap::new();
        if let Some(records) = value.get("records").and_then(Value::as_array) {
            for record in records {
                if let (Some(level), Some(hp)) = (
                    record
                        .get("level")
                        .and_then(|level| int_value(level.get("value"))),
                    int_value(record.get("baseMaxHp")),
                ) {
                    map.insert(level, hp);
                }
            }
        }
        map
    });
    levels
        .get(&level)
        .copied()
        .ok_or_else(|| format!("level {level} is missing from shared/data/level-config.json"))
}

fn decode_recent_battle_bin(bytes: &[u8]) -> Result<Value, String> {
    if bytes.is_empty() {
        return Err("battle record is empty".to_string());
    }
    decode_message(bytes, RECENT_BATTLE_INFO)
}

// ---- Minimal protobuf wire decoder -----------------------------------------

#[derive(Clone, Copy)]
struct FieldSpec {
    number: u32,
    name: &'static str,
    kind: Kind,
}

#[derive(Clone, Copy)]
enum Kind {
    Bool,
    Identifier,
    Int,
    Int64,
    IntMap,
    Nested(&'static [FieldSpec]),
    PackedInt,
    SelectionList,
    String,
    TalentDataMap,
}

enum WireValue {
    Varint(u64),
    Bytes(Vec<u8>),
}

struct WireField {
    number: u32,
    wire_type: u8,
    value: WireValue,
}

const SELECTION_DATA: &[FieldSpec] = &[
    FieldSpec {
        number: 1,
        name: "id",
        kind: Kind::Int,
    },
    FieldSpec {
        number: 2,
        name: "pendings",
        kind: Kind::PackedInt,
    },
    FieldSpec {
        number: 3,
        name: "selected",
        kind: Kind::Int,
    },
];

const BATTLE_TALENT_DATA: &[FieldSpec] = &[FieldSpec {
    number: 1,
    name: "commonParams",
    kind: Kind::PackedInt,
}];

const FATE_STRATEGY_DATA: &[FieldSpec] = &[
    FieldSpec {
        number: 1,
        name: "strategies",
        kind: Kind::SelectionList,
    },
    FieldSpec {
        number: 2,
        name: "counters",
        kind: Kind::IntMap,
    },
];

const LAST_ROUND: &[FieldSpec] = &[
    FieldSpec {
        number: 1,
        name: "life",
        kind: Kind::Int,
    },
    FieldSpec {
        number: 2,
        name: "extraMaxHp",
        kind: Kind::Int,
    },
    FieldSpec {
        number: 3,
        name: "exp",
        kind: Kind::Int,
    },
    FieldSpec {
        number: 4,
        name: "level",
        kind: Kind::Int,
    },
    FieldSpec {
        number: 5,
        name: "talents",
        kind: Kind::PackedInt,
    },
    FieldSpec {
        number: 6,
        name: "usedCards",
        kind: Kind::PackedInt,
    },
    FieldSpec {
        number: 7,
        name: "handCards",
        kind: Kind::PackedInt,
    },
    FieldSpec {
        number: 8,
        name: "talentTempDatas",
        kind: Kind::IntMap,
    },
    FieldSpec {
        number: 9,
        name: "permanentBuffTempDatas",
        kind: Kind::IntMap,
    },
    FieldSpec {
        number: 10,
        name: "unlockGrids",
        kind: Kind::Int,
    },
    FieldSpec {
        number: 15,
        name: "usedKeYinCards",
        kind: Kind::PackedInt,
    },
    FieldSpec {
        number: 16,
        name: "fateStrategies",
        kind: Kind::PackedInt,
    },
    FieldSpec {
        number: 17,
        name: "xianMoStrategies",
        kind: Kind::PackedInt,
    },
];

const PUBLIC_PLAYER: &[FieldSpec] = &[
    FieldSpec {
        number: 1,
        name: "uid",
        kind: Kind::Identifier,
    },
    FieldSpec {
        number: 2,
        name: "username",
        kind: Kind::String,
    },
    FieldSpec {
        number: 4,
        name: "extraMaxHp",
        kind: Kind::Int,
    },
    FieldSpec {
        number: 5,
        name: "exp",
        kind: Kind::Int,
    },
    FieldSpec {
        number: 6,
        name: "level",
        kind: Kind::Int,
    },
    FieldSpec {
        number: 8,
        name: "career",
        kind: Kind::Int,
    },
    FieldSpec {
        number: 12,
        name: "characterId",
        kind: Kind::Int,
    },
    FieldSpec {
        number: 13,
        name: "talents",
        kind: Kind::PackedInt,
    },
    FieldSpec {
        number: 14,
        name: "talentTempDatas",
        kind: Kind::IntMap,
    },
    FieldSpec {
        number: 17,
        name: "permanentBuffTempDatas",
        kind: Kind::IntMap,
    },
    FieldSpec {
        number: 22,
        name: "talentDatas",
        kind: Kind::TalentDataMap,
    },
    FieldSpec {
        number: 200,
        name: "lastRound",
        kind: Kind::Nested(LAST_ROUND),
    },
];

const XIAN_MO_DATA: &[FieldSpec] = &[
    FieldSpec {
        number: 1,
        name: "strategies",
        kind: Kind::SelectionList,
    },
    FieldSpec {
        number: 2,
        name: "counters",
        kind: Kind::IntMap,
    },
    FieldSpec {
        number: 3,
        name: "tempDatas",
        kind: Kind::IntMap,
    },
];

const PRIVATE_PLAYER: &[FieldSpec] = &[
    FieldSpec {
        number: 1,
        name: "handCards",
        kind: Kind::PackedInt,
    },
    FieldSpec {
        number: 2,
        name: "usedCards",
        kind: Kind::PackedInt,
    },
    FieldSpec {
        number: 3,
        name: "unlockGrids",
        kind: Kind::Int,
    },
    FieldSpec {
        number: 10,
        name: "talentDatas",
        kind: Kind::TalentDataMap,
    },
    FieldSpec {
        number: 19,
        name: "fateStrategyData",
        kind: Kind::Nested(FATE_STRATEGY_DATA),
    },
    FieldSpec {
        number: 20,
        name: "xianMoData",
        kind: Kind::Nested(XIAN_MO_DATA),
    },
];

const PLAYER_DATA: &[FieldSpec] = &[
    FieldSpec {
        number: 1,
        name: "public",
        kind: Kind::Nested(PUBLIC_PLAYER),
    },
    FieldSpec {
        number: 2,
        name: "private",
        kind: Kind::Nested(PRIVATE_PLAYER),
    },
];

const BATTLE_RESULT: &[FieldSpec] = &[
    FieldSpec {
        number: 1,
        name: "p1",
        kind: Kind::Nested(PLAYER_DATA),
    },
    FieldSpec {
        number: 2,
        name: "p2",
        kind: Kind::Nested(PLAYER_DATA),
    },
    FieldSpec {
        number: 3,
        name: "battleParams",
        kind: Kind::PackedInt,
    },
    FieldSpec {
        number: 4,
        name: "actorTurnCount",
        kind: Kind::Int,
    },
    FieldSpec {
        number: 5,
        name: "hpDelta",
        kind: Kind::Int,
    },
    FieldSpec {
        number: 6,
        name: "lifeDamage",
        kind: Kind::Int,
    },
    FieldSpec {
        number: 7,
        name: "round",
        kind: Kind::Int,
    },
    FieldSpec {
        number: 8,
        name: "firstPlayerId",
        kind: Kind::Identifier,
    },
    FieldSpec {
        number: 9,
        name: "winnerId",
        kind: Kind::Identifier,
    },
    FieldSpec {
        number: 16,
        name: "gameMode",
        kind: Kind::Int,
    },
    FieldSpec {
        number: 17,
        name: "subMode",
        kind: Kind::Int,
    },
    FieldSpec {
        number: 18,
        name: "seasonMechanism",
        kind: Kind::Int,
    },
];

const RECENT_BATTLE_INFO: &[FieldSpec] = &[
    FieldSpec {
        number: 2,
        name: "gameMode",
        kind: Kind::Int,
    },
    FieldSpec {
        number: 4,
        name: "beginTs",
        kind: Kind::Int64,
    },
    FieldSpec {
        number: 5,
        name: "endTs",
        kind: Kind::Int64,
    },
    FieldSpec {
        number: 8,
        name: "battleRank",
        kind: Kind::Int,
    },
    FieldSpec {
        number: 11,
        name: "version",
        kind: Kind::String,
    },
    FieldSpec {
        number: 18,
        name: "uid",
        kind: Kind::Identifier,
    },
    FieldSpec {
        number: 20,
        name: "codeId",
        kind: Kind::Int64,
    },
    FieldSpec {
        number: 24,
        name: "isVersusPractice",
        kind: Kind::Bool,
    },
    FieldSpec {
        number: 25,
        name: "subMode",
        kind: Kind::Int,
    },
    FieldSpec {
        number: 32,
        name: "xianFu",
        kind: Kind::Bool,
    },
    FieldSpec {
        number: 34,
        name: "script",
        kind: Kind::Int,
    },
    FieldSpec {
        number: 100,
        name: "roundStats",
        kind: Kind::Nested(BATTLE_RESULT),
    },
];

fn decode_message(bytes: &[u8], schema: &[FieldSpec]) -> Result<Value, String> {
    let mut object = Map::new();
    for field in parse_message(bytes)? {
        let Some(spec) = schema.iter().find(|spec| spec.number == field.number) else {
            continue;
        };
        match spec.kind {
            Kind::Bool => {
                let value = int32(varint_field(&field)?);
                object.insert(spec.name.to_string(), json!(value != 0));
            }
            Kind::Int => {
                let value = int32(varint_field(&field)?);
                object.insert(spec.name.to_string(), json!(value));
            }
            Kind::Int64 => {
                let value = varint_field(&field)?;
                object.insert(spec.name.to_string(), json!(value as i64));
            }
            Kind::Identifier | Kind::String => {
                let value = bytes_field(&field)?;
                let text = std::str::from_utf8(value)
                    .map_err(|error| format!("field {} is not UTF-8: {error}", spec.name))?;
                object.insert(spec.name.to_string(), json!(text));
            }
            Kind::PackedInt => {
                let values = decode_packed_ints(bytes_field(&field)?)?;
                object.insert(spec.name.to_string(), json!(values));
            }
            Kind::IntMap => {
                let (key, value) = decode_int_map(bytes_field(&field)?)?;
                let entry = object
                    .entry(spec.name.to_string())
                    .or_insert_with(|| json!({}));
                let map = entry.as_object_mut().ok_or("int map is not an object")?;
                map.insert(key.to_string(), json!(value));
            }
            Kind::Nested(nested) => {
                let decoded = decode_message(bytes_field(&field)?, nested)?;
                if spec.name == "roundStats" {
                    let entry = object
                        .entry(spec.name.to_string())
                        .or_insert_with(|| json!([]));
                    let list = entry.as_array_mut().ok_or("roundStats is not an array")?;
                    list.push(decoded);
                } else {
                    object.insert(spec.name.to_string(), decoded);
                }
            }
            Kind::SelectionList => {
                let decoded = decode_message(bytes_field(&field)?, SELECTION_DATA)?;
                let entry = object
                    .entry(spec.name.to_string())
                    .or_insert_with(|| json!([]));
                let list = entry
                    .as_array_mut()
                    .ok_or("selection list is not an array")?;
                list.push(decoded);
            }
            Kind::TalentDataMap => {
                let decoded = decode_talent_data_map(bytes_field(&field)?)?;
                let entry = object
                    .entry(spec.name.to_string())
                    .or_insert_with(|| json!([]));
                let list = entry.as_array_mut().ok_or("talent map is not an array")?;
                list.push(decoded);
            }
        }
    }
    Ok(Value::Object(object))
}

fn parse_message(bytes: &[u8]) -> Result<Vec<WireField>, String> {
    let mut fields = Vec::new();
    let mut offset = 0usize;
    while offset < bytes.len() {
        let tag = read_varint(bytes, &mut offset)?;
        let number = u32::try_from(tag >> 3).map_err(|_| "protobuf field number overflow")?;
        let wire_type = (tag & 7) as u8;
        if number == 0 {
            return Err("protobuf field number 0 is invalid".to_string());
        }
        match wire_type {
            0 => {
                let value = read_varint(bytes, &mut offset)?;
                fields.push(WireField {
                    number,
                    wire_type,
                    value: WireValue::Varint(value),
                });
            }
            1 => {
                skip(bytes, &mut offset, 8)?;
            }
            2 => {
                let length = read_varint(bytes, &mut offset)?;
                let length = usize::try_from(length).map_err(|_| "protobuf field is too large")?;
                let end = offset
                    .checked_add(length)
                    .filter(|end| *end <= bytes.len())
                    .ok_or("protobuf field runs past the end of the record")?;
                fields.push(WireField {
                    number,
                    wire_type,
                    value: WireValue::Bytes(bytes[offset..end].to_vec()),
                });
                offset = end;
            }
            5 => {
                skip(bytes, &mut offset, 4)?;
            }
            other => return Err(format!("unsupported protobuf wire type {other}")),
        }
    }
    Ok(fields)
}

fn read_varint(bytes: &[u8], offset: &mut usize) -> Result<u64, String> {
    let mut value = 0u64;
    let mut shift = 0u32;
    while *offset < bytes.len() && shift < 70 {
        let byte = bytes[*offset];
        *offset += 1;
        value |= u64::from(byte & 0x7f) << shift;
        if byte < 0x80 {
            return Ok(value);
        }
        shift += 7;
    }
    Err("battle record contains a corrupt protobuf varint".to_string())
}

fn skip(bytes: &[u8], offset: &mut usize, size: usize) -> Result<(), String> {
    *offset = offset
        .checked_add(size)
        .filter(|end| *end <= bytes.len())
        .ok_or("protobuf field runs past the end of the record")?;
    Ok(())
}

fn varint_field(field: &WireField) -> Result<u64, String> {
    match (&field.value, field.wire_type) {
        (WireValue::Varint(value), 0) => Ok(*value),
        _ => Err(format!("field {} is not a varint", field.number)),
    }
}

fn bytes_field(field: &WireField) -> Result<&[u8], String> {
    match (&field.value, field.wire_type) {
        (WireValue::Bytes(value), 2) => Ok(value),
        _ => Err(format!("field {} is not length-delimited", field.number)),
    }
}

fn int32(value: u64) -> i64 {
    (value as u32 as i32) as i64
}

fn decode_packed_ints(bytes: &[u8]) -> Result<Vec<i64>, String> {
    let mut values = Vec::new();
    let mut offset = 0usize;
    while offset < bytes.len() {
        values.push(int32(read_varint(bytes, &mut offset)?));
    }
    Ok(values)
}

fn decode_int_map(bytes: &[u8]) -> Result<(i64, i64), String> {
    let mut key = 0i64;
    let mut value = 0i64;
    for field in parse_message(bytes)? {
        match field.number {
            1 => key = int32(varint_field(&field)?),
            2 => value = int32(varint_field(&field)?),
            _ => {}
        }
    }
    Ok((key, value))
}

fn decode_talent_data_map(bytes: &[u8]) -> Result<Value, String> {
    let mut id = 0i64;
    let mut params = Map::new();
    for field in parse_message(bytes)? {
        match field.number {
            1 => id = int32(varint_field(&field)?),
            2 => {
                params = decode_message(bytes_field(&field)?, BATTLE_TALENT_DATA)?
                    .as_object()
                    .cloned()
                    .unwrap_or_default();
            }
            _ => {}
        }
    }
    let mut object = Map::new();
    object.insert("id".to_string(), json!(id));
    for (key, value) in params {
        object.insert(key, value);
    }
    Ok(Value::Object(object))
}

// ---- JSON helpers ----------------------------------------------------------

fn int_value(value: Option<&Value>) -> Option<i64> {
    let value = value?;
    value
        .as_i64()
        .or_else(|| value.as_u64().and_then(|number| i64::try_from(number).ok()))
}

fn int_or(value: Option<&Value>, default: i64) -> i64 {
    int_value(value).unwrap_or(default)
}

fn int_list(value: Option<&Value>) -> Vec<i64> {
    value
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| int_value(Some(item)))
                .collect()
        })
        .unwrap_or_default()
}

fn string_value(value: Option<&Value>) -> String {
    value
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string()
}

fn number_map(value: Option<&Value>) -> BTreeMap<String, i64> {
    let Some(map) = value.and_then(Value::as_object) else {
        return BTreeMap::new();
    };
    map.iter()
        .filter_map(|(key, value)| int_value(Some(value)).map(|value| (key.clone(), value)))
        .collect()
}

fn side_for_uid(
    value: &str,
    p1_uid: &str,
    p2_uid: &str,
    label: &str,
) -> Result<&'static str, String> {
    if !value.is_empty() && value == p1_uid {
        return Ok("p1");
    }
    if !value.is_empty() && value == p2_uid {
        return Ok("p2");
    }
    Err(format!("{label} ID does not belong to either player"))
}

fn normalize_base_id(card_id: i64) -> i64 {
    if card_id == 0 || card_id == 10_000 || card_id == 20_000 || card_id == 1_000_000 {
        return 0;
    }
    card_id - ((card_id % 1_000_000) / 10_000) * 10_000
}

#[cfg(test)]
mod tests {
    use super::*;

    fn put_varint(out: &mut Vec<u8>, mut value: u64) {
        while value >= 0x80 {
            out.push((value as u8 & 0x7f) | 0x80);
            value >>= 7;
        }
        out.push(value as u8);
    }

    fn field_varint(out: &mut Vec<u8>, number: u32, value: u64) {
        put_varint(out, (u64::from(number) << 3) | 0);
        put_varint(out, value);
    }

    fn field_bytes(out: &mut Vec<u8>, number: u32, value: &[u8]) {
        put_varint(out, (u64::from(number) << 3) | 2);
        put_varint(out, value.len() as u64);
        out.extend_from_slice(value);
    }

    fn packed(values: &[u64]) -> Vec<u8> {
        let mut out = Vec::new();
        for value in values {
            put_varint(&mut out, *value);
        }
        out
    }

    fn player(uid: &str, card_ids: &[u64]) -> Vec<u8> {
        let mut last_round = Vec::new();
        field_varint(&mut last_round, 4, 1);
        field_bytes(&mut last_round, 6, &packed(card_ids));

        let mut public = Vec::new();
        field_bytes(&mut public, 1, uid.as_bytes());
        field_varint(&mut public, 12, 1);
        field_bytes(&mut public, 200, &last_round);

        let mut private = Vec::new();
        field_bytes(&mut private, 2, &packed(card_ids));
        field_varint(&mut private, 3, DECK_SLOTS as u64);

        let mut out = Vec::new();
        field_bytes(&mut out, 1, &public);
        field_bytes(&mut out, 2, &private);
        out
    }

    fn battle(round: u64, winner: &str, cards: &[u64]) -> Vec<u8> {
        let mut out = Vec::new();
        field_bytes(&mut out, 1, &player("u1", cards));
        field_bytes(&mut out, 2, &player("u2", cards));
        field_bytes(&mut out, 3, &packed(&[1, 2, 3]));
        field_varint(&mut out, 4, 5);
        field_varint(&mut out, 5, 4);
        field_varint(&mut out, 7, round);
        field_bytes(&mut out, 8, b"u1");
        field_bytes(&mut out, 9, winner.as_bytes());
        out
    }

    #[test]
    fn decodes_bin_record_and_builds_fixture() {
        let mut record = Vec::new();
        field_bytes(&mut record, 11, b"test-version");
        field_bytes(&mut record, 100, &battle(3, "u1", &[1000001; DECK_SLOTS]));

        let decoded = decode_recent_battle_bin(&record).expect("record decodes");
        assert_eq!(decoded["version"], "test-version");
        let battle = &decoded["roundStats"][0];
        assert_eq!(battle["winnerId"], "u1");
        assert_eq!(battle["actorTurnCount"], 5);

        let fixture = build_fixture(&decoded, battle).expect("fixture builds");
        assert_eq!(fixture["schemaVersion"], 1);
        assert_eq!(fixture["firstPlayerSide"], "p1");
        assert_eq!(fixture["expected"]["winnerSide"], "p1");
        assert_eq!(fixture["expected"]["actorTurnCount"], 5);
        assert_eq!(fixture["expected"]["hpDeltaP1MinusP2"], 4);
        assert_eq!(fixture["players"]["p1"]["baseMaxHp"], 40);
        assert_eq!(
            fixture["players"]["p1"]["cards"].as_array().unwrap().len(),
            8
        );
        assert_eq!(fixture["decisionTape"], json!([1, 2, 3]));

        let parsed: BattleFixture =
            serde_json::from_value(fixture.clone()).expect("fixture contract accepts output");
        assert_eq!(parsed.players.p1.base_max_hp, 40);
    }

    #[test]
    fn adapts_mirror_record_fields_to_fixture() {
        let record = json!({
            "data": {
                "version": "mirror-version",
                "seasonMec": 7,
                "roundStats": [{
                    "p1": {
                        "publicData": {
                            "uid": "p1-uid",
                            "characterId": 1001,
                            "lastRoundData": {
                                "level": 2,
                                "usedCards": [1000001, 1000001, 1000001, 1000001, 1000001, 1000001, 1000001, 1000001],
                                "handCards": [1000001, null],
                                "talents": [11],
                                "talentTempDatas": {"3": 4},
                                "permanentBuffTempDatas": {"5": 6}
                            },
                            "talentDatas": {"11": {"commonParams": [9]}}
                        },
                        "privateData": {
                            "usedCards": [1, 2, 3],
                            "unlockGrids": 8,
                            "fateStrategyData": {
                                "strategies": [{"id": 1, "selected": 2}],
                                "tempDatas": {"12": 13}
                            },
                            "xianMoData": {"strategies": [{"selected": 7}], "tempDatas": {"8": 9}}
                        }
                    },
                    "p2": {
                        "publicData": {
                            "uid": "p2-uid",
                            "characterId": 1002,
                            "lastRoundData": {
                                "level": 2,
                                "usedCards": [1000001, 1000001, 1000001, 1000001, 1000001, 1000001, 1000001, 1000001]
                            }
                        },
                        "privateData": {"usedCards": [1, 2, 3], "unlockGrids": 8}
                    },
                    "winerId": "p2-uid",
                    "huiHeCount": 12,
                    "hpDelta": -8,
                    "round": 4,
                    "firstPlayerId": "p1-uid",
                    "battleParams": [4, 5]
                }]
            }
        });

        let records = normalize_json_records(record).expect("mirror record normalizes");
        let battle = &records[0]["roundStats"][0];
        assert_eq!(battle["winnerId"], "p2-uid");
        assert_eq!(battle["actorTurnCount"], 12);
        assert_eq!(battle["seasonMechanism"], 7);

        let fixture = build_fixture(&records[0], battle).expect("mirror fixture builds");
        assert_eq!(fixture["expected"]["winnerSide"], "p2");
        assert_eq!(fixture["expected"]["actorTurnCount"], 12);
        assert_eq!(fixture["expected"]["hpDeltaP1MinusP2"], -8);
        assert_eq!(fixture["players"]["p1"]["baseMaxHp"], 45);
        assert_eq!(
            fixture["players"]["p1"]["lastRoundUsedCardBaseIds"][0],
            1000001
        );
        assert_eq!(
            fixture["players"]["p1"]["talentCardParams"]["11"],
            json!([9])
        );
        assert_eq!(fixture["players"]["p1"]["xianMoStrategies"], json!([7]));
        assert_eq!(fixture["players"]["p1"]["xianMoTempDatas"]["8"], 9);
        assert_eq!(fixture["players"]["p1"]["fateStrategyTempDatas"]["12"], 13);
        assert_eq!(fixture["players"]["p1"]["fateStrategies"], Value::Null);
    }

    #[test]
    fn normalizes_base_ids() {
        assert_eq!(normalize_base_id(0), 0);
        assert_eq!(normalize_base_id(10_000), 0);
        assert_eq!(normalize_base_id(20_000), 0);
        assert_eq!(normalize_base_id(1_000_000), 0);
        assert_eq!(normalize_base_id(1000001), 1000001);
        assert_eq!(normalize_base_id(1010028), 1000028);
    }

    #[test]
    fn parses_jsonl_input() {
        let line = json!({"version": "v", "roundStats": []}).to_string();
        let bytes = format!("{line}\n{line}\n");
        let records =
            decode_input(bytes.as_bytes(), Path::new("input.jsonl")).expect("jsonl parses");
        assert_eq!(records.len(), 2);
    }
}
