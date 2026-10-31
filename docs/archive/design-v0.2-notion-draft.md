# tesh プロジェクト設計案 v0.2（Notion 原案）

> **出典・位置づけ（アーカイブ）**
> - 出典: Notion「tesh プロジェクト設計案 v0.2」 https://app.notion.com/p/32e6a05ca4d380a1a003e9426351bba3?pvs=204 の全文転記（書式のみ Markdown に整えた）。
> - 本書は [../design.md](../design.md) の元になった原案である。design.md は本書を整理・再編集した現行の正であり、本書とは節構成・番号が異なる。内容が食い違う場合は design.md が優先する。
> - design.md に引き継がれなかった内容（差分記法の例、move コマンド、移行戦略、README 向け文案など）を含むため、原案としてそのまま保存する。

---

## 1. 目的

tesh は、Terraform/OpenTofu のための**環境差分編集レイヤ**である。

Terraform の実行モデルは変えず、複数環境・検証環境・派生環境を **Git のような差分ワークフロー** で扱えるようにする。

tesh が解く問題：

- 環境差分が `tfvars`、ディレクトリ規約、wrapper script に散らばる
- 「この環境は何を試しているか」が読めない
- branch だけでは安全に並行環境を扱えない
- 検証環境を丸ごと複製・apply・destroy しづらい
- 標準構成からの差分を昇格・比較しづらい

tesh の役割：

- **land**: 標準構成
- **shrub**: 差分パッチ
- **env**: land + shrubs の適用結果として存在する環境
- **output**: build された完全な Terraform/OpenTofu 生成物

tesh は Terraform を置き換えない。

**Terraform は実行、tesh は編集。**

---

# 2. 設計原則

## 原則1: Terraform の挙動は変えない

- build 後の成果物は通常の `.tf`
- plan/apply/destroy の意味は Terraform/OpenTofu に委ねる
- state, graph, provider semantics は Terraform 側の責務

## 原則2: 環境差分は branch ではなく env と shrub で扱う

- Git branch は履歴管理
- tesh env は並行環境管理
- shrub は land に対する差分パッチ

## 原則3: 差分は意味単位として読めるべき

- 「何を試しているか」が shrub 名と内容で分かる
- diff は値差分だけでなく構成差分として表現される

## 原則4: 検証環境は簡単に生やせて簡単に捨てられるべき

- `tesh newenv` で標準構成から派生環境を作る
- build + apply で即座に環境確認できる
- destroy まで含めてワークフロー化する

## 原則5: 差分は昇格できるべき

- shrub は一時検証にも永続差分にも使える
- 検証済み差分は land へ promote できる

## 原則6: 競合はテキストより手前で見つける

- Git はファイル競合を扱う
- tesh は構成競合を扱う
- 同じ module の同じ属性に対する複数 shrub の上書きは検知対象

---

# 3. 概念モデル

## 3.1 land

標準構成。

すべての env の基底となる。

役割：

- 現行の安定版構成
- 共通部品
- 基本形
- promote の最終着地点

Git 比喩：

- `main` / `default branch`

---

## 3.2 shrub

land に対する差分パッチ。

機能追加、設定変更、削除、実験、一時検証、移行用変更などを表す。

役割：

- 差分の作業単位
- 検証対象
- 変更の意味単位

例：

- `apcpuv4`
- `waf-v2`
- `egress-lockdown`
- `alb-timeout-90`

Git 比喩：

- patch
- feature branch の差分部分
- cherry-pick 可能な変更セット

### shrub の性質

- 差分だけ書いてもよい
- land と同じ記法で丸ごと書いてもよい
- 必ずしも永続機能ではない
- promote / move / squash 対象になりうる

---

## 3.3 env

land + 0個以上の shrubs を適用した結果として存在する環境定義。

役割：

- 並行運用される実体
- build/apply 対象
- backend/state/account などの文脈を束ねる単位

例：

- `prod`
- `stg`
- `test-ApCPUv4`
- `dr-gcp-candidate`

Git 比喩：

- branch に少し似るが、履歴ではなく実行対象の世界線

---

## 3.4 output

build によって生成される完成 Terraform/OpenTofu 構成。

人が読める・差分比較できる・破棄再生成できる成果物。

役割：

- 実行入力
- レビュー対象
- 監査対象
- 再現可能な生成物

---

# 4. ディレクトリ構成案

```txt
project/
  tesh.toml

  land/
    config.toml
    modules/
      app.toml
      db.toml
    terraform/
      providers.tf
      versions.tf
      backend.tf

  shrubs/
    apcpuv4/
      shrub.toml
      modules/
        app.toml

    waf-v2/
      shrub.toml
      modules/
        waf.toml

    alb-timeout-90/
      shrub.toml
      modules/
        app.toml

  envs/
    prod/
      env.toml

    stg/
      env.toml

    test-ApCPUv4/
      env.toml

  output/
    prod/
    stg/
    test-ApCPUv4/
```

---

# 5. 設定ファイル案

## 5.1 `tesh.toml`

プロジェクト全体設定。

```toml
name = "sample-platform"
terraform_binary = "terraform"
output_dir = "output"

[defaults]
strict = true
state_mode = "remote_only"
```

---

## 5.2 `land/config.toml`

land のメタ情報。

```toml
name = "default-land"
description = "Stable baseline infrastructure"
```

---

## 5.3 `shrubs/<name>/shrub.toml`

shrub のメタ情報。

```toml
name = "apcpuv4"
description = "Upgrade app CPU generation for verification"
type = "experiment"

[metadata]
owner = "gomi"
purpose = "Performance verification"
ttl = "7d"
```

`type` 例:

- `experiment`
- `feature`
- `migration`
- `hotfix`
- `override`

---

## 5.4 `envs/<name>/env.toml`

env 定義。

```toml
name = "test-ApCPUv4"
base = "land"

shrubs = ["apcpuv4"]

[context]
provider = "aws"
account = "sandbox"
region = "ap-northeast-1"
backend = "s3://my-tfstate/test-ApCPUv4"

[metadata]
purpose = "Verify CPU generation upgrade"
based_on = "prod"
```

---

# 6. 差分記法の方針

## 6.1 基本思想

shrub は **land と同じ記法** で書ける。

ただしその意味は「定義」ではなく「差分適用」。

つまり同じ `modules/app.toml` でも：

- land では完全定義
- shrub では override/patch

として扱う。

---

## 6.2 例: land 側

```toml
[module]
name = "app"
source = "../modules/app"

[module.inputs]
instance_type = "c7g.large"
desired_count = 2
timeout = 30
```

---

## 6.3 例: shrub 側（差分だけ）

```toml
[module]
name = "app"

[module.inputs]
instance_type = "c7g.xlarge"
```

意味：

- `app` module の `instance_type` だけ上書き

---

## 6.4 例: shrub 側（より広い上書き）

```toml
[module]
name = "app"
source = "../modules/app"

[module.inputs]
instance_type = "c7g.xlarge"
desired_count = 4
timeout = 45
```

---

# 7. マージ仕様 v0.1

ここは最重要。曖昧禁止。

## 7.1 適用順序

1. land
2. env 固有 override
3. shrubs（env.toml に並んだ順）
4. CLI override（将来）

※ shrubs 順序は明示的。暗黙順禁止。

---

## 7.2 map/object

- **deep merge**
- 同じキーがあれば後勝ち

## 7.3 scalar

- 後勝ち

## 7.4 list

初期仕様では **replace**

- append/merge はやらない
- 複雑化回避のため将来検討

## 7.5 delete

初期仕様では特殊記法で明示。

```toml
[module.inputs]
timeout = { __delete__ = true }
```

もしくはもっと綺麗な専用構文は後で検討。

## 7.6 unknown target

- 存在しない module/resource を shrub が override したら error
- strict=false のとき warning に落とす余地あり

## 7.7 conflict

同一 env で複数 shrub が同じ属性を上書きした場合：

- デフォルト warning
- strict=true なら error 可能
- `tesh explain` で最終値の由来を追跡可能にする

---

# 8. コマンド体系案

## 8.1 環境作成

```bash
tesh newenv test-ApCPUv4 --from prod
```

意味：

- `prod` ベースで新 env 作成
- metadata もコピー可能

---

## 8.2 shrub 作成

```bash
tesh newshrub apcpuv4 --type experiment
```

---

## 8.3 shrub 追加

```bash
tesh env add-shrub test-ApCPUv4 apcpuv4
```

---

## 8.4 build

```bash
tesh build test-ApCPUv4
```

結果：

- `output/test-ApCPUv4/` に完成 Terraform を生成

---

## 8.5 apply

```bash
tesh apply test-ApCPUv4
```

内部的には：

- 必要なら build
- `output/test-ApCPUv4/` で terraform apply

※ v0.1 では `tesh apply` を持つかは要議論。

思想としては `tesh build && terraform apply` の分離もあり。

---

## 8.6 diff

```bash
tesh diff prod test-ApCPUv4
```

表示レイヤ：

1. shrub 差分
2. config 差分
3. generated tf 差分

---

## 8.7 explain

```bash
tesh explain test-ApCPUv4 module.app.inputs.instance_type
```

出力例：

```txt
final value: "c7g.xlarge"
source chain:
  land/modules/app.toml -> "c7g.large"
  shrubs/apcpuv4/modules/app.toml -> "c7g.xlarge"
```

---

## 8.8 promote

```bash
tesh promote shrub apcpuv4 --to land
```

意味：

- shrub 差分を land へ取り込む
- shrub は archive or keep を選択

---

## 8.9 move

```bash
tesh move shrub apcpuv4 --to shared/perf
```

※ v0.2 以降でもよい

---

## 8.10 doctor

```bash
tesh doctor
```

チェック：

- 未解決 override
- conflict
- 不要 shrub
- ttl 切れ実験 shrub
- remote state 方針違反

---

# 9. 安全性方針

## 9.1 env は実行文脈を持つ

env ごとに最低限以下を明示：

- provider
- account/project
- region
- backend/state location

これにより apply 先を構造的に分離する。

## 9.2 prod 保護

`env.metadata.class = "prod"` の場合：

- apply に確認を要求
- destroy は二重確認
- CI only 実行など将来対応

## 9.3 remote state only

ローカル state 原則禁止。

## 9.4 output は破棄再生成可能

手編集禁止。

生成物に provenance コメントを埋める。

---

# 10. 移行戦略

## フェーズ1

既存 Terraform root module を land に寄せる。

まずは shrub なしでもよい。

価値：

- env を明示化できる
- output が生成できる

## フェーズ2

既存の差分を shrub と env override に切り出す。

例：

- 追加機能 → shrub
- 値変更のみ → env override

## フェーズ3

promote/move/squash ワークフロー導入。

## フェーズ4

将来的に抽象 module / multi-cloud 同等性へ拡張余地を残す。

ただし v0.x の主戦場はここではない。

---

# 11. 非目標

tesh がやらないこと：

- Terraform state の独自管理
- Terraform graph の置き換え
- provider semantics の置き換え
- plan/apply ロジックの独自化
- 完全なマルチクラウド等価性保証
- 実行オーケストレーション全般の覇権化

tesh は **環境差分編集フレームワーク** に徹する。

---

# 12. 価値提案の一言

## 短い版

**Terraform 環境を、Git のように差分で扱う。**

## もう少し説明する版

**land を標準構成、shrub を差分パッチとして、検証環境・派生環境を安全に build/apply できる Terraform 編集レイヤ。**

## README 冒頭向け

**Git は履歴を管理する。Terraform は実行する。tesh は環境差分を管理する。**

---

# 13. v0.1 で絶対に必要なもの

ここは削れない。

1. land / shrub / env の厳密定義
2. deterministic build
3. 明文化された merge 仕様
4. `newenv`
5. `build`
6. `diff`
7. `explain`
8. env context の明示
9. conflict 検出
10. remote state only

---

# 14. v0.1 でやらない方がいいもの

欲張ると死ぬ。

- マルチクラウド抽象 resource
- list の賢い merge
- 実行 DAG の大規模制御
- hooks 地獄
- 独自 DSL の作り込み
- Terraform variable schema 自動生成の深入り

---

# 15. 最終まとめ

今の tesh は、こう定義するのが一番強い。

**tesh は Terraform/OpenTofu のための Git ライクな環境差分フレームワークである。**

**land を標準構成、shrub を差分パッチ、env を並行運用される実行対象として扱い、検証・比較・昇格を環境単位で可能にする。**

**Terraform の実行モデルは変えず、構成の編集モデルだけを拡張する。**

---

必要なら次、これをさらに詰められる。

おすすめはこの2つ。

1. **この設計案を Notion にそのまま貼れる体裁に整える**
2. **v0.1 の CLI 仕様とファイル仕様だけに絞って、実装可能レベルまで落とす**

正直、次は 2 が熱い。

ここまで来たら、気持ちいい思想をちゃんと実装可能な穴の少ない形に締めた方がいい。
