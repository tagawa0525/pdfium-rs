# PR #1-15 レビューコメント未対応修正

## Context

PR #1〜#15（Phase 1-2）に対するCopilotレビューコメントの反映漏れを調査した結果、
セキュリティ/正確性に関する7件、堅牢性2件、ドキュメント4件の計13件が未対応であった。
これらを一括で修正し、コードベースの品質を担保する。

## スコープ外

- **PR #14-1: 暗号化PDF内の文字列復号** — 新機能。Phase 3で対応。
- **PR #14-2: 暗号化統合テストの文字列検証** — PR #14-1に依存。
- **PR #12-4 (W1): `revision6_hash()`のループ終端条件** — 調査の結果、既に`loop` + `i >= 64 && (i - 32) >= last_byte`で正しく実装済み。

## 計画書ファイル名

`docs/plans/003_review-fixes-pr1-15.md`（実行開始時にリネーム＋コミット）

## ブランチ

`fix/review-fixes-pr1-15`

## コミット構成

レビュー修正は`fix(<scope>):`コミット（CLAUDE.mdルール）。RED/GREENサイクルは不要。
テストは修正と同一コミットに含める。

### Group 1: security.rs（C1, C2, C3）

**Commit 1**: `fix(fpdfapi/security): validate user_hash length before slice access`

- `src/fpdfapi/parser/security.rs`
- `check_user_password()` (L249): `dict.user_hash[..16]` アクセス前に長さチェック追加
- `user_hash.len() < 16` → `None` を返す（R2は32バイト全比較なので影響なし）
- テスト: `check_user_password_short_user_hash_returns_none`
- Refs: PR #12 comment

**Commit 2**: `fix(fpdfapi/security): clamp key_length in calc_encrypt_key`

- `src/fpdfapi/parser/security.rs`
- `calc_encrypt_key()` (L246, L265): `key_length`を`.clamp(1, 16)`で制限
- MD5出力は16バイト固定。PDF仕様上key_lengthは5-16。超過時のパニックを防止
- テスト: `calc_encrypt_key_oversized_key_length_is_clamped`
- Refs: PR #12 comment

**Commit 3**: `fix(fpdfapi/security): propagate AES error in revision6_hash`

- `src/fpdfapi/parser/security.rs`
- `revision6_hash()` (L478): `unwrap_or(content)` → `Option`返却に変更
- シグネチャ: `fn revision6_hash(...) -> Option<[u8; 32]>`
- 呼び出し元`aes256_check_password()`（4箇所）: `revision6_hash(...)?` で`None`伝播
- AES失敗 → パスワード不一致として処理（既存のOption返却フローと整合）
- Refs: PR #12 comment

### Group 2: encrypt_dict.rs（C4, C5, S3）

**Commit 4**: `fix(fpdfapi/encrypt_dict): reject invalid cipher for V=5 and unknown /V`

- `src/fpdfapi/parser/encrypt_dict.rs`
- `determine_cipher()`: 戻り値を`Cipher` → `Result<Cipher>`に変更
- V=5 non-AESV3 (L128): `Cipher::Aes128` → `Err(InvalidPdf("V=5 requires AESV3"))`
- Unknown /V (L131): `Cipher::None` → `Err(InvalidPdf("unsupported /V"))`
- 呼び出し元 (L32): `determine_cipher(v, dict)` → `determine_cipher(v, dict)?`
- テスト: `determine_cipher_v5_non_aesv3_is_error`, `determine_cipher_unknown_v_is_error`
- Refs: PR #14 comments

**Commit 5**: `fix(fpdfapi/encrypt_dict): correct doc comment to match implementation`

- `src/fpdfapi/parser/encrypt_dict.rs` (L5-9)
- `/StmF`, `/StrF` を読むと記載しているが実装なし → 実態に合わせて修正
- Refs: PR #14 comment

### Group 3: document.rs（C6, C7, W3）

**Commit 6**: `fix(fpdfapi/document): validate /ID presence for encrypted PDFs`

- `src/fpdfapi/parser/document.rs` (L155)
- `extract_file_id()` 呼び出し後に `file_id.is_empty()` チェック追加
- 暗号化PDFで/ID欠落 → `InvalidPdf("encrypted PDF requires /ID in trailer")`
- `extract_file_id()`自体のシグネチャは変更しない（非暗号化文脈に影響しないため）
- Refs: PR #14 comment

**Commit 7**: `fix(fpdfapi/document): verify object number from /Encrypt indirect reference`

- `src/fpdfapi/parser/document.rs` (L138)
- `let (_, obj)` → `let (parsed_id, obj)` + 番号一致検証
- `object()`メソッド (L235-240) と同じパターンを適用
- Refs: PR #14 comment

**Commit 8**: `refactor(fpdfapi/document): use Cow to avoid clone in decode_stream`

- `src/fpdfapi/parser/document.rs` (L185)
- `stream.data.clone()` → `Cow::Borrowed(&stream.data)`
- `decode::decode_stream`は`&[u8]`を受け取るため、`&raw`で透過的に参照
- Refs: PR #14 comment

### Group 4: テスト追加（W2）

**Commit 9**: `test(fpdfapi/security): add Revision 6 password check tests`

- `src/fpdfapi/parser/security.rs` テストモジュール
- `aes256_check_password_r6_user`: R5テストと同構造、`revision: 6`で`revision6_hash`コードパスを検証
- 既存`aes256_check_password_r5_user`をベースに作成
- Refs: PR #12 comment

### Group 5: ドキュメント修正（S1, S2, S4）

**Commit 10**: `docs: fix cipher descriptions in Phase 2 plan`

- `docs/plans/002_phase2-encryption-and-stream-decode.md`
- L80: `PDF 2.0 の暗号化` → `PDF 1.7 ExtL3+ / PDF 2.0 の暗号化`
- L99: `| 5 (AES) | AES-128 | PDF 1.6+ |` → `| 5 | AES-256 | PDF 1.7 ExtL3+ |`
- Refs: PR #6 comments

**Commit 11**: `docs: update CLAUDE.md error handling to reflect Phase 2 status`

- `CLAUDE.md` (L52-53)
- `Phase 2以降: thiserror構造化enumに移行` → Phase 2はstd::error::Errorで完了した事実を反映
- Refs: PR #3 comment

## 検証

各コミット後:

```bash
cargo check --all-features && cargo test --all-features && cargo clippy --all-features --all-targets -- -D warnings && cargo fmt --all -- --check
```

## 対象ファイル

| ファイル                                                | コミット   |
| ------------------------------------------------------- | ---------- |
| `src/fpdfapi/parser/security.rs`                        | 1, 2, 3, 9 |
| `src/fpdfapi/parser/encrypt_dict.rs`                    | 4, 5       |
| `src/fpdfapi/parser/document.rs`                        | 6, 7, 8    |
| `docs/plans/002_phase2-encryption-and-stream-decode.md` | 10         |
| `CLAUDE.md`                                             | 11         |
