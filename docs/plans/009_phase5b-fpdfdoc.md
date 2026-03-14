# Phase 5b: fpdfdoc — ブックマーク・注釈・フォーム（読み取り専用）

## Context

Phase 5（テキスト描画）が完了し、レンダリングパイプラインが一通り動作する状態。
次に fpdfdoc モジュールを実装し、PDF ドキュメントレベルの構造情報（ブックマーク、注釈、リンク、フォーム）への読み取りアクセスを提供する。

C++ PDFium の `core/fpdfdoc/` に対応。読み取り専用のデータ抽出に絞り、外観生成（`CPDF_GenerateAP` 1650行）やフォーム入力は対象外。

### スコープ

**対象**: ブックマーク/アウトライン、デスティネーション、アクション、注釈、リンク、フォームフィールド（読み取り）、名前ツリー
**対象外**: 外観ストリーム生成、フォーム入力、JavaScript 実行、Type3/CID フォント、構造ツリー（アクセシビリティ）

---

## 設計方針

### ボロー問題の解決

`Document::object(&mut self)` は遅延パースのため `&mut self` を要求する。fpdfdoc 型は `Document` への参照を保持できない。

**解決策**: `TextPage::build()` や `DocumentInfo` と同じ「抽出→所有」パターンを採用。`Document` のメソッドで必要な辞書データをすべて解決・クローンし、所有型として返す。さらに参照解決が必要な場合（名前付きデスティネーション等）は `&mut Document` を引数に取るメソッドを提供する。

### モジュール構造

```text
src/fpdfdoc/
  mod.rs             # モジュール宣言・再エクスポート
  dest.rs            # デスティネーション（ズームモード・座標）
  action.rs          # アクション型（GoTo, URI, Named 等）
  bookmark.rs        # ブックマーク/アウトラインツリー
  annot.rs           # 注釈型・サブタイプ
  link.rs            # リンク注釈ラッパー
  form.rs            # AcroForm フィールド（読み取り専用）
  name_tree.rs       # 名前ツリー走査ユーティリティ
```

### 外部依存

追加なし。すべて既存の `PdfObject`/`PdfDictionary` 上の辞書走査で完結する。

---

## ステップ分割

### Step 1: モジュール scaffold + Dest 型

**ブランチ**: `feat/fpdfdoc-dest`
**C++ 対応**: `CPDF_Dest`（49行）

**型**:

```rust
// src/fpdfdoc/dest.rs
pub enum ZoomMode {
    Unknown, XYZ, Fit, FitH, FitV, FitR, FitB, FitBH, FitBV,
}

pub struct Dest {
    pub page_index: Option<u32>,   // 0-based
    pub zoom_mode: ZoomMode,
    pub params: Vec<f32>,          // ズームモード後のパラメータ
}

impl Dest {
    pub fn from_array(arr: &[PdfObject], page_index: Option<u32>) -> Self;
    pub fn xyz(&self) -> Option<(Option<f32>, Option<f32>, Option<f32>)>;
}
```

**変更ファイル**:

- 新規: `src/fpdfdoc/mod.rs`, `src/fpdfdoc/dest.rs`
- 変更: `src/lib.rs` — `pub mod fpdfdoc;` 追加

**テスト**: 合成 `PdfObject::Array` で全ズームモード、null パラメータ、不正配列

---

### Step 2: Action 型

**ブランチ**: `feat/fpdfdoc-action`
**C++ 対応**: `CPDF_Action`（79行 h, 236行 cpp）
**依存**: Step 1

**型**:

```rust
// src/fpdfdoc/action.rs
pub enum ActionType {
    Unknown, GoTo, GoToR, GoToE, Launch, Uri, Named,
    JavaScript, SubmitForm, ResetForm, ImportData,
    Hide, Sound, Movie, Thread, SetOcgState,
    Rendition, Trans, GoTo3DView,
}

pub struct Action {
    dict: PdfDictionary,
}

impl Action {
    pub fn from_dict(dict: PdfDictionary) -> Self;
    pub fn action_type(&self) -> ActionType;       // /S → enum
    pub fn uri(&self) -> Option<String>;            // /URI
    pub fn named_action(&self) -> Option<String>;   // /N
    pub fn javascript(&self) -> Option<String>;     // /JS (文字列のみ、ストリームは後)
    pub fn file_path(&self) -> Option<String>;      // /F
    pub fn sub_actions(&self) -> Vec<Action>;       // /Next
}
```

`Action::dest()` は名前付きデスティネーションの解決に `Document` が必要なため Step 5 で追加。

**テスト**: `cpdf_action_unittest.cpp` 移植 — /S 文字列→ActionType 変換、URI/Named 抽出

---

### Step 3: Bookmark ツリー

**ブランチ**: `feat/fpdfdoc-bookmark`
**C++ 対応**: `CPDF_Bookmark`（36行）, `CPDF_BookmarkTree`（27行）
**依存**: Step 2

**型**:

```rust
// src/fpdfdoc/bookmark.rs
pub struct Bookmark {
    pub title: String,
    pub action: Option<Action>,
    pub dest_array: Option<Vec<PdfObject>>,  // インラインデスト
    pub count: i32,                          // 負=閉じた状態
    pub children: Vec<Bookmark>,
}
```

**API**: `Document::bookmarks(&mut self) -> Result<Vec<Bookmark>>`

- `/Root → /Outlines → /First` から `/First`/`/Next` チェーンを再帰走査
- `HashSet<u32>` で循環参照を検出（`bookmarks_circular.pdf` 対策）
- `/Title` の制御文字をスペースに置換（C++ 同等処理）

**テスト**: 空ドキュメント、単一ブックマーク、ネスト、循環参照検出
**統合テスト**: `bookmarks.pdf` フィクスチャ

---

### Step 4: Annotation 型 + page_dict 取得

**ブランチ**: `feat/fpdfdoc-annot`
**C++ 対応**: `CPDF_Annot`（subtype/rect/flags）, `CPDF_AnnotList`
**依存**: Step 2

**前提変更**: `Document` にページ辞書を返す内部メソッドを追加。

```rust
// document.rs に追加
fn find_page_dict_in_tree(...) -> Result<Option<(PdfDictionary, PageInherit)>>;
```

既存の `find_page_in_tree` のロジックを流用し、`build_page` を呼ばずに辞書を返す。`page()` は `find_page_dict_in_tree` + `build_page` にリファクタリング。

**型**:

```rust
// src/fpdfdoc/annot.rs
pub enum AnnotSubtype {
    Unknown, Text, Link, FreeText, Line, Square, Circle,
    Polygon, Polyline, Highlight, Underline, Squiggly, Strikeout,
    Stamp, Caret, Ink, Popup, FileAttachment, Sound, Movie,
    Widget, Screen, PrinterMark, TrapNet, Watermark, ThreeD,
    RichMedia, Redact,
}

pub struct AnnotFlags(u32);
impl AnnotFlags {
    pub fn invisible(&self) -> bool;
    pub fn hidden(&self) -> bool;
    pub fn print(&self) -> bool;
    // ... 他のフラグ
}

pub struct Annotation {
    pub subtype: AnnotSubtype,
    pub rect: Rect,
    pub flags: AnnotFlags,
    pub contents: Option<String>,
    pub name: Option<String>,
    pub modified: Option<String>,
    pub action: Option<Action>,
    pub dict: PdfDictionary,  // 高度なアクセス用
}
```

**API**: `Document::page_annotations(&mut self, page_index: u32) -> Result<Vec<Annotation>>`

- ページ辞書の `/Annots` 配列を走査、各参照を解決してクローン

**テスト**: サブタイプ文字列→enum 変換（27種）、フラグビット抽出、空 /Annots
**統合テスト**: `annots.pdf` フィクスチャ

---

### Step 5: NameTree + Dest 解決 + Link

**ブランチ**: `feat/fpdfdoc-nametree`
**C++ 対応**: `CPDF_NameTree`（走査のみ、読み取り専用）, `CPDF_Link`
**依存**: Step 1, Step 4

**型**:

```rust
// src/fpdfdoc/name_tree.rs
pub struct NameTree;
impl NameTree {
    /// 名前ツリー内の値を検索（/Names + /Kids 再帰走査）
    pub fn lookup(
        doc: &mut Document<impl Read + Seek>,
        root: &PdfDictionary,
        name: &[u8],
    ) -> Result<Option<PdfObject>>;

    /// 名前付きデスティネーションを検索（/Names/Dests + 旧式 /Dests フォールバック）
    pub fn lookup_named_dest(
        doc: &mut Document<impl Read + Seek>,
        name: &[u8],
    ) -> Result<Option<Vec<PdfObject>>>;
}
```

```rust
// src/fpdfdoc/link.rs
pub struct Link {
    pub rect: Rect,
    pub dest: Option<Dest>,
    pub action: Option<Action>,
}
```

**追加メソッド**:

- `Action::dest(&self, doc: &mut Document<R>) -> Result<Option<Dest>>` — `/D` の解決
- `Document::page_links(&mut self, page_index: u32) -> Result<Vec<Link>>`

**テスト**: 合成名前ツリー（`/Names`直接 + `/Kids`ネスト）、旧式 `/Dests` フォールバック
**統合テスト**: `named_dests.pdf`, `goto_action.pdf`, `uri_action.pdf`

---

### Step 6: InteractiveForm（読み取り専用）

**ブランチ**: `feat/fpdfdoc-form`
**C++ 対応**: `CPDF_InteractiveForm`, `CPDF_FormField`, `CPDF_FormControl`
**依存**: Step 2

**型**:

```rust
// src/fpdfdoc/form.rs
pub enum FormFieldType {
    Unknown, PushButton, CheckBox, RadioButton,
    Text, RichText, File, ListBox, ComboBox, Signature,
}

pub struct FormOption {
    pub label: String,
    pub value: String,
}

pub struct FormField {
    pub full_name: String,         // ドット区切り階層名
    pub field_type: FormFieldType,
    pub value: Option<String>,     // /V
    pub default_value: Option<String>, // /DV
    pub flags: u32,                // /Ff
    pub options: Vec<FormOption>,  // /Opt（選択肢フィールド用）
    pub selected_indices: Vec<i32>, // /I
    pub max_len: Option<i32>,      // /MaxLen
    pub alternate_name: Option<String>, // /TU（ツールチップ）
    pub read_only: bool,
    pub required: bool,
}

pub struct InteractiveForm {
    pub fields: Vec<FormField>,
}
```

**API**: `Document::form(&mut self) -> Result<Option<InteractiveForm>>`

- `/Root → /AcroForm → /Fields` を再帰走査
- `/FT` と `/Ff` は親から継承（`GetFieldAttrRecursive` パターン）
- `full_name` は `/Parent` チェーンの `/T` をドット連結
- `/Btn` + flags でフィールド型判別（PushButton/CheckBox/RadioButton）

**テスト**: フィールド型判定（Btn+flags→PushButton/CheckBox/RadioButton）、階層名連結、選択肢抽出
**統合テスト**: `text_form.pdf`, `multiple_form_types.pdf`, `combobox_form.pdf`, `listbox_form.pdf`

---

### Step 7: 統合テスト + 公開 API

**ブランチ**: `feat/fpdfdoc-integration`
**依存**: Step 1-6

**変更**:

- `tests/fixtures/` にテスト PDF をコピー:
  - `bookmarks.pdf`, `annots.pdf`, `text_form.pdf`, `named_dests.pdf`, `goto_action.pdf`, `uri_action.pdf`
- `tests/integration_fpdfdoc.rs` 作成 — `fpdf_doc_embeddertest.cpp` からテスト移植
- `src/lib.rs` に再エクスポート追加:

  ```rust
  pub use fpdfdoc::{
      Annotation, AnnotSubtype, AnnotFlags, Bookmark,
      Dest, ZoomMode, Action, ActionType,
      FormField, FormFieldType, InteractiveForm, Link,
  };
  ```

- `docs/plans/` ステータス更新

---

## 依存グラフ

```text
Step 1 (Dest)
  ↓
Step 2 (Action) ← Dest 参照
  ↓
  ├→ Step 3 (Bookmark) ← Action
  ├→ Step 4 (Annotation) ← Action + page_dict リファクタ
  ├→ Step 6 (Form) ← Action
  ↓
Step 5 (NameTree + Link) ← Dest, Action, Annotation
  ↓
Step 7 (統合テスト) ← 全ステップ
```

Step 3, 4, 6 は Step 2 完了後に独立して着手可能。

## 重要ファイル

| ファイル                         | 役割                                                       |
| -------------------------------- | ---------------------------------------------------------- |
| `src/fpdfapi/parser/document.rs` | `bookmarks()`, `page_annotations()`, `form()` メソッド追加 |
| `src/fpdfapi/parser/object.rs`   | `PdfDictionary` アクセサ（既存で十分）                     |
| `src/lib.rs`                     | 公開 API 再エクスポート                                    |
| `src/fpdftext/text_page.rs`      | 「抽出→所有」パターンの参考実装                            |

## リスク

1. **循環参照**: ブックマークツリーで発生しうる → `HashSet<u32>` で検出
2. **フォームフィールドツリーの複雑さ**: 属性継承と型判定が最も複雑な部分 → C++ の `GetFieldAttrRecursive` パターンを忠実に移植
3. **名前ツリーの深さ**: 大きな PDF では `/Kids` が深くネストする → 再帰上限を設ける

## 検証方法

```bash
cargo test --all-features
cargo clippy --all-features --all-targets -- -D warnings
cargo fmt --all -- --check

# 統合テスト
cargo test fpdfdoc
cargo test integration_fpdfdoc
```

## 進捗

| Step | 内容                          | 状態 |
| ---- | ----------------------------- | ---- |
| 1    | Dest 型 + モジュール scaffold | todo |
| 2    | Action 型                     | todo |
| 3    | Bookmark ツリー               | todo |
| 4    | Annotation + page_dict        | todo |
| 5    | NameTree + Link               | todo |
| 6    | InteractiveForm (読み取り)    | todo |
| 7    | 統合テスト + 公開 API         | todo |
