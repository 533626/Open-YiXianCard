/**
 * 开放状态标记锚点检查（check-doc-drift 的一部分）。
 *
 * 反例驱动：文档里写过「留作待办」「未修」「尚未定位」却没有任何命令/路径/链接锚点，
 * 换人接手无法核实现状——209 卡点「已解除 vs 留作待办」矛盾、1097→1228 构筑数漂移
 * 都属于这类。规则：
 *
 * - 行内标记（留作待办 / TODO / FIXME / 未修 / 尚未定位 / 尚未完成 / 尚未接入 /
 *   待接入）所在行必须含反引号代码段或 markdown 链接，否则报违规；
 * - 「## 下一步」类标题段落（到下一个同级或更高级标题为止）必须至少含一个锚点。
 *
 * 纯「待办」「下一步」等宽泛词不在此列（"待办批次"是描述不是开放项）；代码围栏内的
 * 内容跳过。
 */

export const OPEN_STATUS_MARKERS =
  /(留作待办|TODO|FIXME|未修|尚未定位|尚未完成|尚未接入|待接入)/;

const ANCHOR = /`[^`\n]+`|\[[^\]]+\]\([^)\n]+\)/;
const NEXT_STEP_HEADER = /^#{1,6}\s*下一步/;
const FENCE = /^```/;

export interface OpenItemViolation {
  /** 1-based 行号 */
  readonly line: number;
  readonly kind: "inline" | "section";
  readonly marker: string;
}

export function findUnanchoredOpenItems(content: string): OpenItemViolation[] {
  const lines = content.split("\n");
  const violations: OpenItemViolation[] = [];
  let inFence = false;

  for (let i = 0; i < lines.length; i++) {
    const line = lines[i]!;
    if (FENCE.test(line)) {
      inFence = !inFence;
      continue;
    }
    if (inFence) continue;

    if (NEXT_STEP_HEADER.test(line)) {
      const level = (line.match(/^(#+)/)![1]!).length;
      const block: string[] = [line];
      for (let j = i + 1; j < lines.length; j++) {
        const next = lines[j]!;
        const header = next.match(/^(#+)\s/);
        if (header && header[1]!.length <= level) break;
        block.push(next);
      }
      if (!block.some((l) => ANCHOR.test(l))) {
        violations.push({ line: i + 1, kind: "section", marker: "下一步" });
      }
    } else {
      const match = line.match(OPEN_STATUS_MARKERS);
      if (match !== null && !ANCHOR.test(line)) {
        violations.push({ line: i + 1, kind: "inline", marker: match[1]! });
      }
    }
  }

  return violations;
}
