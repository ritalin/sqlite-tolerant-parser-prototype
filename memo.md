bindgen-cliで警告抑制

bindgen <PATH> -o <OUT-PATH> --raw-line '#![allow(dead_code, non_snake_case, non_camel_case_types, non_upper_case_globals)]'


BUILD_DIR=build cargo run --bin grammar_converter --features grammar_converter > build/grammar.json  
BUILD_DIR=build cargo run --bin keyword_matcher --features keyword_matcher > build/keywords.h  

# scanner

1. leading trivia
2. キーワード / 演算子
3. 文字列リテラル / 数値リテラル
4. 識別子
5. trailing trivia

## scanner (lexme)
キーワードを1文字目でグルーピング
グループは長さの降順
順にstr::eq_ignore_ascii_case()でチェック
最初に見つかったものを返す。
グループはphfでマッピング
グループは、トークンとトークン長の組で持たせる

## scanner (leading trivia)
ブロックコメント + 空白
ラインコメント +空白
正規表現検索
r"(?s)/\*.*?\*/"
r"\s+"

## scanner (trailing trivia)
空白(スペース / 水平タブ / 改行)を正規表現検索

## scanner (文字列リテラル)
"か'で始まってたら正規表現検索

## scanner (数値リテラル)
正規表現検索

## scanner (識別子)
正規表現検索

escapeどうする？

-----

# ファイル作成
tempfile crateで裏で作っておいて、最後に差し替え

# 構文木
woran crate

# LALR(1)

https://web.archive.org/web/20210507215636/https://web.cs.dal.ca/~sjackson/lalr1.html

Rule

0. S → N
1. N → V = E
2. N → E
3. E → V
4. V → x
5. V → * E

Transition Table

| Item Set |  x  |  =  |  *  |  S  |  N  |  E  |  V  |
+----------+-----+-----+-----+-----+-----+-----+-----+
| 0        | 1   |     | 2   |     | 4   | 5   | 3   |
| 1        |     |     |     |     |     |     |     |
| 2        | 1   |     | 2   |     |     | 6   | 7   |
| 3        |     | 8   |     |     |     |     |     |
| 4        |     |     |     |     |     |     |     |
| 5        |     |     |     |     |     |     |     |
| 6        |     |     |     |     |     |     |     |
| 7        |     |     |     |     |     |     |     |
| 8        | 1   |     | 2   |     |     | 9   | 7   |
| 9        |     |     |     |     |     |     |     |

Extended Grammar

| rule |      grammar      |
+------+-------------------+
| 0    | 0S$ → 0N4         |
| 1    | 0V3 → 0x1         |
| 2    | 0V3 → 0*2 2E6     |
| 3    | 0N4 → 0E5         |
| 4    | 0N4 → 0V3 3=8 8E9 |
| 5    | 0E5 → 0V3         |
| 6    | 2E6 → 2V7         |
| 7    | 2V7 → 2x1         |
| 8    | 2V7 → 2*2 2E6     |
| 9    | 8V7 → 8x1         |
| 10   | 8V7 → 8*2 2E6     |
| 11   | 8E9 → 8V7         |

First Set (Shiftに使う)

First(0S$) = First(0N4) = First(0E5) = First(0V3) = { x, * } 

Follow Set (Reduce判定に使う)

| rule |      grammar      |        follow        |                    path                     |
+------+-------------------+----------------------+---------------------------------------------+
| 0    | 0S$ → 0N4         | Follow(0N4) = {$}    | 0N4 -> 0S$                                  |
| 1    | 0V3 → 0x1         | Follow(0x1) = {}     | -                                           |
| 2    | 0V3 → 0*2 2E6     | Follow(0*2) = {}     | -                                           |
|      |                   | Follow(2E6) = {$, =} | 2E6 -> 0V3 -> 0N4, 2E6 -> 0V3 -> 0N4 -> 0S$ |
| 3    | 0N4 → 0E5         | Follow(0E5) = {$}    | 0E5 -> 0N4 -> 0S$                           |
| 4    | 0N4 → 0V3 3=8 8E9 | Follow(0V3) = {=}    | -                                           |
|      |                   | Follow(3=8) = {}     | -                                           |
|      |                   | Follow(8E9) = {$}    | 8E9 -> 0N4 -> 0S$                           |
| 5    | 0E5 → 0V3         | Follow(0V3) = {$}    | 0V3 -> 0E5 -> 0N4 -> 0S$                    |
| 6    | 2E6 → 2V7         | Follow(2V7) = {$, =} | 2V7 -> 2E6 -> (rule#4)                      |
| 7    | 2V7 → 2x1         | Follow(2x1) = {}     | -                                           |
| 8    | 2V7 → 2*2 2E6     | Follow(2*2) = {}     | -                                           |
|      |                   | Follow(2E6) = {}     | 2E6 -> 2V7                                  |
| 9    | 8V7 → 8x1         | Follow(8x1) = {}     | -                                           |
| 10   | 8V7 → 8*2 2E6     | Follow(8*2) = {}     | -                                           |
|      |                   | Follow(2E6) = {}     | 2E6 -> 8V7                                  |
| 11   | 8E9 → 8V7         | Follow(8V7) = {$}    | 8V7 -> 8E9 ->  0N4 -> 0S$ ????              |

Goto Table

## reduce_on

precedenceルールは非終端 -> 終端の順に探索する。
探索できるように構築する必要がある。

% left A B

のように同一トークンに同一スコアを提示する場合、結合による評価を下す。

```rust
fn reduce_on(&self, lookahead: &str, rhs: &[Symbol]) -> bool {
    let la_prec = self.precedence.get(lookahead);
    // FIXME: 右から見て最初に見つかったprecedenceルールを採用する
    let rhs_prec = rhs.iter().rev() // 最も右側の終端トークン
        .filter_map(|sym| match sym {
            Symbol::Terminal(name) => self.precedence.get(name),
            _ => None,
        })
        .next()

    match (la_prec, rhs_prec) {
        (Some(la), Some(rhs)) => {
            if rhs.level > la.level {
                // Reduceの方が強い
                true
            } else if rhs.level < la.level {
                // Lookaheadの方が強い（Shift）
                false
            } else {
                // 優先度同じ → 結合性で決定
                match rhs.associativity {
                    Assoc::Left => true,   // Reduce優先
                    Assoc::Right => false, // Shift優先
                    Assoc::NonAssoc => {
                        // 非結合の場合はconflictとみなしてfalseにするのが安全
                        false
                    }
                }
            }
        }
        // 優先度が定義されていない場合はデフォルトtrue（reduce）
        _ => true,
    }
}
```

## prioryty_of

まずrhsのprecedenceルールを評価 -> 存在すれば採用
ない場合はlookaheadのprecedenceルールを評価

precedenceルールは非終端 -> 終端の順に探索する。
探索できるように構築する必要がある。

結合ルールの種類の違いによる判定は不要。
そもそも同じスコアで異なる結合を指定できないため。
結合ルールの種類の違いはShift/Reduce conflictの判定に重要
Reduce/Reduce conflictでは不要

## translation

遷移表の入力はlookaheadのみ。scanしたトークンを常にlookaheadと想定して渡すだけで済む。

## error recovery

`Corchuelo et al. algorithm`をベースに1トークン削除と、一定深さのシフトのみ行う。
シフトは幅優先で、状態が一つ進むごとにdepth+1して閾値で打ち切り

* queue要素としてnext_stateとstate_routeを保持
    * ~~state_routeに再ヒットしたら打ち切り~~
    * スキップ中もスタック増減のシミュレーションを兼ねる
        * 確定後再実行してスタック差分を計算
* ~~別のShiftアクションにヒットしたら打ち切り~~

Unrecoverableの場合はSEMI or EOFまでscanしてErrorノードに持たせる。
Recoverableの場合は、スキップした状態をErrorノードに持たせる。
貢献するreduceが打開できない場合は、Errorノードを挿入（valueにはlookahead）し、lookaheadを進める(再Shift)。

~~Errorノードはスタックに積まず別途Errorベクタで管理する。~~
Errorノードもスタックに持たせるが、reduceの際にカウント対象外で強制的に含める
* 貢献あるreduceでノードスタックとマージして子要素にする
    * [x] ノード種別の解決の際に注釈を見て子ノードをソートする
        * まだ順序がバラバラなままなので

lookaheadのトランザクション管理が必要。

少なくとも1回状態スタックを動かすredeceまで観察する。

セミコロンやEOFなどの終端で打ち切り、"statement"ノードでまとめる。
Acceptには決して到達しない。

* セミコロン直後のEOFは別"statement"ノードに置く
    * セミコロンがなければ前の"statement"に食わせる
* セミコロンレスも自然に対応できる。
    * セミコロンがどこからreduceし始めるのか
        * おそらくはセミコロンがshiftするまで進める必要あり
    * ecmd := cmdx EOFのルール追加で解決する余地はある

セミコロン以降仕切り直しでリスタート。
EOFまで終わったところで、すべての"statement"ノードを束ね"program"ノードを作る。
終端やサブルート、ルートノードはConfigで指定する。

スコアリングはより深く進んだ結果を採用

## include!()で条件コンパイル

build.rs
```rs
use std::env;
use std::fs;

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();

    // 存在確認
    let use_extra = fs::metadata("src/extra_impl.rs").is_ok();

    if use_extra {
        println!("cargo:rustc-cfg=has_extra_impl");
    }
    else {
        println!("cargo:warning=Code generation is required for extra implementation.");
    }
}
```

main.rs
```rs
#[cfg(has_extra_impl)]
include!("extra_impl.rs");

#[cfg(not(has_extra_impl))]
mod fallback {
    pub fn do_something() {
        println!("default fallback impl");
    }
}
```

~~TK_STAR~~がキーワードになってる？なんで？
~~TK_COMMA~~
~~DOT~~
~~CONCAT~~
EOF
~~SPACE~~

## incremental parsing

置き換える範囲を元に、その最末端までの範囲を入力に拡張する。
後ろはどこに繋げるかわからないから

前は1byte拡張し前の兄弟をスタックに積む
* 子ノードのソートでrangeが必要なためRedNodeから構築する必要あり

`SELECT 123 FROM foo`を`SELECT 123 barFROM foo`と変更す流場合、変更箇所だけだと`FROM`の破損を捉えられない。
そのため、前後1byteずつ広げる。

拡張後の範囲の確定は、内包する共通の祖先を巻き込ませる。
triviaはスキップして拡張する？

`cstree`は異なる種別のGreenNodeのreplaceを直接サポートしてない。
差し替え対象の親のGreenノードを取得
子ノードをVecに詰め込む。
`Vec::splice()`で新しいノードと差し替える。
新しい親ノードを作成する。
RedNodeで差し替える。

差分パースでもエラーが解消しない場合に適切にツリーを構築するかどうか
マルチステートメントにまたがる修正

# wasm32-wasi

`./wit/world.wit`を作る。
`resource`は`world`で直接`export`できる。
`world`名はインポート側からは見えないため、定型的な名前にしておけば良い(`<SOME-COMPONENT>-world`等)
`resource`が引数や戻り値で`record`に依存していても`world`でも明示的な`export`は不要。
ただし、ほかのコンポーネントからインポートする場合は、`record`や`interface`を明示的に`export`する必要がある。

* `record`を`world`で直接エクスポートすることはできない
    * 必ず`interface`下に置かなければならない

## wit-bindgen

* `generate`マクロ、はコンポーネントをインポートし、そのインターフェースを実装するために使用するもの（クライアント）。
* `export`マクロは、コンポーネントの実装を公開(`export`)するためのもの（サーバ）。
    * サーバで`generate`マクロを使ってもタグジャンプできない。
    * wit-bindgen-cliで明示的に作成する必要がある。

```
wit-bindgen rust \
    --async none \
    --out-dir src/bindings \
    --default-bindings-module "crate::bindings::scanner_world" \
    --world scanner-world \
    ./wit
```

wit-bindgen rust \
    --async none \
    --out-dir src/bindings \
    --default-bindings-module "crate::bindings::scanners" \
    --world scanners \
    ./wit

wit-bindgen rust \
    --async none \
    --out-dir src/bindings \
    --default-bindings-module "crate::bindings::types" \
    --world types \
    ./wit


CSTは以下の理由から`resource`が望ましそう
* Idを持つ
* 生殺与奪の権理は受けとった側が持つ
* それなりに大きくなる
* 基本read-onlyなので`RefCell`ではなく`Rc`での保持がベターそう

## client

ローカルパスのwitをインポートする場合、`wit/deps.toml`を用意し

```
<WORLD名> = <相対PATH>
```

で指定する。
次いで、`wit-deps update`でコンポーネントの依存を追加する。

規定のパス以外を対象にする場合は以下の全てを明示に指定する必要がある

* `-m`で`deps.toml`のパス
    * 規定値は`./wit/deps.toml`
* `-l`でロックファイルのパス
    * 規定値は`./wit/deps.lock`
* `-d`で出力先
    * 規定値は`./wit/deps`

`generate`マクロでインターフェースを展開する場合、以下のいずれかの指定を要求される。

* `generate_all`
* `with { "<PREFIX>:<PACKAGE>/<INTERFACE>": generate }`
* `with { "<PREFIX>:<PACKAGE>/<INTERFACE>/<TYPE>": <MOD-path> }`
    * `MOD-path`は`crate::`からの完全名で指定すること

## ビルド

cargo --bin scanner_app --package scaanner-wasi --target wasm32-wasip2 --release
cargo --package scaanner-wasi --target wasm32-wasip2 --release

## がっちゃんこ

wac plug target/wasm32-wasip2/release/scanner_app.wasm --plug target/wasm32-wasip2/release/scanner_wasi.wasm -o target/wasm32-wasip2/release/scanner.wasm

## browserで表示

`jco`でバインディングを作る

```
# wasmから型定義の出力なしでtranspile
pnpm exec jco transpile pkg/wasm32-wasip2/release/scanner_wasi.wasm -o pkg/scanner --name scanner --no-typescript 
# witから型定義を生成
pnpm exec jco types ../../crates/scanner-wasi/wit/world.wit --name scanner -o pkg/scanner 
```

## parser

```
wit-bindgen rust \
    --async none \
    --out-dir src/bindings \
    --default-bindings-module "crate::bindings::parser_world" \
    --world parser-world \
    --with ritalin:scanner/types@0.0.1="scanner_wasi::scanner_types" \
    ./wit
```

wit-bindgen rust \
    --async none \
    --out-dir src/bindings \
    --default-bindings-module "crate::bindings::syntax" \
    --world syntax-interface \
    --with ritalin:scanner/types@0.0.1="scanner_wasi::scanner_types" \
    ./wit


## parser/app

```
wit-deps \
   -m src/bin/parser_app/assets/wit/deps.toml \
   -l src/bin/parser_app/assets/wit/deps.lock \
   -d src/bin/parser_app/assets/wit/deps \
   update
```

wit-bindgen rust \
    --world "app-world",
    path: "src/bin/parser_app/assets/wit",
    default_bindings_module: "crate::api::bindings::app_world",
    generate_all,

`scanner-wasi`は`parser-wasi`が取り込んで`export`しているため、`plug`不要

wac plug target/wasm32-wasip2/release/parser_app.wasm \
    --plug target/wasm32-wasip2/release/parser_wasi.wasm \
    -o target/wasm32-wasip2/release/parser_app_final.wasm