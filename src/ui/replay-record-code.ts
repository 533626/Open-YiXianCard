const RECORD_CODE_XOR = 0x5ff17843b6b1fn;

export function normalizeRecordCode(code: string): string {
  return code.trim().toLowerCase().replace(/\s+/g, "");
}

export function decryptDisplayRecordCode(code: string): string | null {
  const normalized = normalizeRecordCode(code);
  if (!/^[0-9a-z]+$/.test(normalized)) return null;
  try {
    const decoded = reverseRecordNumber(base36ToBigInt(normalized) ^ RECORD_CODE_XOR);
    return toBase36(decoded);
  } catch {
    return null;
  }
}

export function replayMatchesRecordCode(
  replay: Readonly<{ recordCodes: readonly string[] }>,
  code: string,
): boolean {
  const normalized = normalizeRecordCode(code);
  if (!normalized) return true;
  const decrypted = decryptDisplayRecordCode(normalized);
  return replay.recordCodes.includes(normalized) ||
    (decrypted !== null && replay.recordCodes.includes(decrypted));
}

export function recordCodePair(
  codeId: bigint,
  round: number,
  rank: number,
): readonly string[] {
  const number = codeId * 1000n + BigInt(round) * 10n + BigInt(rank);
  return [toBase36(number), toBase36(reverseRecordNumber(number) ^ RECORD_CODE_XOR)];
}

function reverseRecordNumber(value: bigint): bigint {
  const text = value.toString();
  if (text.length <= 1) return value;
  return BigInt(text[0]! + [...text.slice(1)].reverse().join(""));
}

function base36ToBigInt(value: string): bigint {
  let result = 0n;
  for (const char of value) {
    const digit = parseInt(char, 36);
    if (!Number.isInteger(digit) || digit < 0 || digit >= 36) {
      throw new Error("战绩码格式无效");
    }
    result = result * 36n + BigInt(digit);
  }
  return result;
}

function toBase36(value: bigint): string {
  return value > 0n ? value.toString(36) : "";
}
