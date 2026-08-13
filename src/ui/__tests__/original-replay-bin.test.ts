import { describe, expect, test } from "bun:test";
import {
  decodeOriginalReplayBin,
} from "../original-replay-bin";
import {
  decryptDisplayRecordCode,
  replayMatchesRecordCode,
} from "../replay-record-code";

describe("原版 RecentBattleInfo 浏览器解码", () => {
  test("从最小 protobuf 记录生成可导入轮次并保留手牌与精确结果", () => {
    const bytes = recentBattleMessage();
    const decoded = decodeOriginalReplayBin(bytes);

    expect(decoded.gameVersion).toBe("001.0007.0009");
    expect(decoded.beginTimestamp).toBe(1_784_910_815_040);
    expect(decoded.rounds).toHaveLength(1);
    const round = decoded.rounds[0]!;
    expect(round.round).toBe(7);
    expect(round.firstPlayerSide).toBe("p2");
    expect(round.winnerSide).toBe("p1");
    expect(round.actorTurnCount).toBe(12);
    expect(round.hpDeltaP1MinusP2).toBe(-5);
    expect(round.fixture.players.p1.handCards).toEqual([10001, 20002]);
    expect(round.fixture.players.p1.cards.map((card) => card.id)).toEqual(
      Array.from({ length: 8 }, () => 0),
    );
    expect(round.fixture.players.p1.permanentBuffTempDatas).toEqual({ "10023": 4 });
    expect(round.fixture.expected).toEqual({
      winnerSide: "p1",
      actorTurnCount: 12,
      hpDeltaP1MinusP2: -5,
    });
  });

  test("展示码与短码都能匹配同一份本机记录", () => {
    const decoded = decodeOriginalReplayBin(recentBattleMessage());
    const shortCode = decoded.recordCodes[0]!;
    const displayCode = decoded.recordCodes[1]!;

    expect(decryptDisplayRecordCode(displayCode)).toBe(shortCode);
    expect(replayMatchesRecordCode(decoded, shortCode)).toBe(true);
    expect(replayMatchesRecordCode(decoded, displayCode.toUpperCase())).toBe(true);
    expect(replayMatchesRecordCode(decoded, "not-the-code")).toBe(false);
  });

  test("空文件和没有轮次的消息明确失败", () => {
    expect(() => decodeOriginalReplayBin(new Uint8Array())).toThrow("文件为空");
    expect(() => decodeOriginalReplayBin(fieldString(11, "001.0007.0009")))
      .toThrow("没有可导入的对局轮次");
  });
});

function recentBattleMessage(): Uint8Array {
  const p1 = playerMessage("p1-uid", 1_000_005);
  const p2 = playerMessage("p2-uid", 2_000_005);
  const battle = message(
    fieldBytes(1, p1),
    fieldBytes(2, p2),
    fieldPacked(3, [3, 1, 4]),
    fieldInt(4, 12),
    fieldInt(5, -5),
    fieldInt(7, 7),
    fieldString(8, "p2-uid"),
    fieldString(9, "p1-uid"),
    fieldInt(18, 9),
  );
  return message(
    fieldInt(4, 1_784_910_815_040),
    fieldInt(5, 1_784_912_671_055),
    fieldInt(8, 2),
    fieldString(11, "001.0007.0009"),
    fieldInt(20, 31_869_845),
    fieldBytes(100, battle),
  );
}

function playerMessage(uid: string, characterId: number): Uint8Array {
  const lastRound = message(
    fieldInt(1, 36),
    fieldInt(2, 10),
    fieldInt(3, 24),
    fieldInt(4, 5),
    fieldPacked(5, [1, 2, 3]),
    fieldPacked(6, [0, 0, 0]),
    fieldPacked(7, [10001, 20002]),
    intMapField(9, 10023, 4),
    fieldInt(10, 8),
  );
  const publicData = message(
    fieldString(1, uid),
    fieldInt(8, 1),
    fieldInt(12, characterId),
    fieldBytes(200, lastRound),
  );
  const privateData = message(
    fieldPacked(2, Array.from({ length: 8 }, () => 0)),
    fieldInt(3, 8),
  );
  return message(fieldBytes(1, publicData), fieldBytes(2, privateData));
}

function intMapField(field: number, key: number, value: number): Uint8Array {
  return fieldBytes(field, message(fieldInt(1, key), fieldInt(2, value)));
}

function fieldString(field: number, value: string): Uint8Array {
  return fieldBytes(field, new TextEncoder().encode(value));
}

function fieldPacked(field: number, values: readonly number[]): Uint8Array {
  return fieldBytes(field, concat(values.map((value) => varint(signedVarint(value)))));
}

function fieldInt(field: number, value: number): Uint8Array {
  return concat([varint(BigInt(field << 3)), varint(signedVarint(value))]);
}

function fieldBytes(field: number, value: Uint8Array): Uint8Array {
  return concat([
    varint(BigInt((field << 3) | 2)),
    varint(BigInt(value.byteLength)),
    value,
  ]);
}

function signedVarint(value: number): bigint {
  return value < 0 ? BigInt.asUintN(64, BigInt(value)) : BigInt(value);
}

function varint(value: bigint): Uint8Array {
  const bytes: number[] = [];
  let remaining = value;
  do {
    let byte = Number(remaining & 0x7fn);
    remaining >>= 7n;
    if (remaining > 0n) byte |= 0x80;
    bytes.push(byte);
  } while (remaining > 0n);
  return Uint8Array.from(bytes);
}

function message(...fields: readonly Uint8Array[]): Uint8Array {
  return concat(fields);
}

function concat(chunks: readonly Uint8Array[]): Uint8Array {
  const result = new Uint8Array(chunks.reduce((total, chunk) => total + chunk.byteLength, 0));
  let offset = 0;
  for (const chunk of chunks) {
    result.set(chunk, offset);
    offset += chunk.byteLength;
  }
  return result;
}
