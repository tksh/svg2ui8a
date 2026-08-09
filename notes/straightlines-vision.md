# Straightlines: 構想メモ

> ⚠️ **このドキュメントは構想段階の個人的な検討メモです。**
> ここに書かれた将来像は svg2ui8a の現行仕様ではありません。正式な実装判断は
> `docs/` 配下の計画と仕様で行います。

---

## 0. 背景

Straightlines は、**意図的な制約を持つベクターグラフィックスを、長期保存・転送・
ハッシュ可能な Web 標準ベースの形式として育てたい**という構想である。

svg2ui8a はその完成形ではない。ただし、SVG を取り込み、バージョン付きの CBOR
中間表現を `.cbor` として保存し、後で RGBA
へ描画する実験場になる。現在の主目的は 「`.cbor` ファイルのバイト列を
`usvg2rgba` に渡し、画像データを得る」ことである。

---

## 1. 名前と考え方

**Straightlines** = 「直線」。

[tksh/Straightlines](https://github.com/tksh/straightlines) と名称および
「制約は弱さではなく力」という考え方を共有する。一方で、既存のプロジェクトが SVG
サブセットによる作図表現を主題とするのに対し、この構想は**保存可能な
ベクター形式**を主題とする。

目標は、何でも表現できる形式ではなく、何を表現しないかが明確で、複数の実装が
同じデータを安全に扱える形式である。

---

## 2. SVG、usvg、CBOR

### 2.1 SVG の難しさ

SVG は人が書きやすい反面、同じ見た目に至る XML、CSS、属性順、空白、単位、参照の
組み合わせが多い。したがって「視覚的に同じ SVG なら必ず同じバイト列」という
前提は置けない。

### 2.2 usvg の役割

`usvg` は SVG の多くの複雑さを解決済みの木構造へ変換する有用なパーサである。
基本図形をパスへ変換し、参照や相対単位を解決する。ただし `usvg::Tree` は Rust の
内部 API であり、任意の視覚的等価性を証明する正準ファイル形式ではない。直接
シリアライズして永続形式にしてはいけない。

### 2.3 svg2ui8a の現在地

svg2ui8a は `cbor-core` 0.10.1 を使い、`usvg::Tree` の**対応サブセットを
パッケージ所有 DTO に投影**して canonical CBOR を出力する。ファイルは次の
バージョン付き包みを持つ。

| 整数キー | 意味                                 |
| -------- | ------------------------------------ |
| `0`      | 形式識別子 `"svg2ui8a/usvg"`         |
| `1`      | 符号なし形式バージョン（現在は `1`） |
| `2`      | そのバージョンの DTO ペイロード      |

`cbor_core` は正準な CBOR バイト列を扱う。これはハッシュ可能性の基礎だが、
それだけで DTO の意味が正しいことは保証しない。デコーダは識別子、バージョン、
必須フィールド、型、数値範囲、対応する描画要素を検証してから `usvg::Tree` を
再構築する。

この形式を読むだけなら、Deno では `Deno.readFile("image.cbor")` で得た
`Uint8Array` をそのまま `usvg2rgba` に渡せばよい。svg2ui8a 自身はファイルパスや
ストレージを扱わない。

---

## 3. 意図的な制約

現行の svg2ui8a は、将来の Straightlines に向けて次の境界を採用する。

- テキスト、フォント、システムフォント探索は扱わない。
- ラスター画像、`<image>`、画像デコーダは扱わない。
- BBox、アニメーション、外部リソースは中間形式へ入れない。
- 出力は PNG などではなく、生の RGBA `Uint8Array` に限る。
- CBOR は canonical CBOR を使い、非正準な入力は拒否する。

そのため `usvg` 0.47.0 と `resvg` 0.47.0 はどちらも `default-features = false`
で使う。これにより text、system-fonts、memmap-fonts、 raster-images 機能を Wasm
に取り込まない。`tiny-skia` は `resvg` が再公開する
ラスタライザとしてのみ使い、svg2ui8a が直接画像形式を扱う理由にはしない。

入力 SVG がテキストまたは画像を含む場合、曖昧な描画結果を返すのではなく
`svg2usvg` が拒否する。この制約は表現力を減らすためではなく、保存された `.cbor`
を後から同じ契約で描画できるようにするためである。

---

## 4. Straightlines との関係

Straightlines は将来、svg2ui8a の DTO をそのまま標準化することを前提にしない。
svg2ui8a の CBOR は usvg と Wasm 描画のための**パッケージ形式**であり、
Straightlines は別途 CDDL、互換性規則、拡張規則、複数言語での検証を備える
**独立した形式仕様**として設計する。

それでも svg2ui8a は重要な検証手段になる。

1. SVG を受け、制約内の描画モデルを作る。
2. バージョン付き canonical-CBOR の `.cbor` を出力する。
3. 同じバイト列を後で読み、RGBA へレンダリングする。
4. ハッシュ、キャッシュ、形式バージョン、デコード失敗の扱いを検証する。

この経験は Straightlines の将来の CDDL と相互運用テストに役立つが、両者の
スキーマは別々に進化できる。

---

## 5. 将来の Straightlines で検討すること

- 線分・曲線・塗り・グラデーションをどこまで含めるか。
- 色空間、単位、透明度、合成モードの最小集合。
- ネスト、再利用、外部参照を許可するか。
- CDDL による正規スキーマとバージョニング規則。
- Rust、JavaScript、Python、Go などでの相互運用テスト。
- 同じ論理データが同じ CBOR バイト列になるための、形式レベルの正準化規則。

テキスト、フォント、ラスター画像、アニメーションを含めるかは将来の設計判断であり、
現在の svg2ui8a の制約を自動的に解除する理由にはならない。

---

## 6. 関連資料

- [RFC 8949: Concise Binary Object Representation (CBOR)](https://www.rfc-editor.org/rfc/rfc8949)
- [RFC 8610: CDDL](https://www.rfc-editor.org/rfc/rfc8610)
- [cbor_core documentation](https://docs.rs/cbor-core/latest/cbor_core/)
- [resvg / usvg](https://github.com/linebender/resvg)
- [tksh/Straightlines](https://github.com/tksh/straightlines)

---

## 7. メモ

- 2026-08-04: 着想。CBOR の永続化可能性に着目。
- 2026-08-10: `cbor_core` を前提に、svg2ui8a の versioned CBOR pipeline と
  Straightlines の独立した将来仕様を区別した。
