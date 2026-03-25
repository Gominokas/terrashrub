# tesh プロジェクト設計案 v0.2

## 1. 目的

tesh は、Terraform/OpenTofu のための環境差分編集レイヤである。
Terraform の実行モデルは変えず、複数環境・検証環境・派生環境を Git のような差分ワークフローで扱えるようにする。

tesh が解く問題：

- 環境差分が tfvars、ディレクトリ規約、wrapper script に散らばる
- この環境は何を試しているかが読めない
- branch だけでは安全に並行環境を扱えない
- 検証環境を丸ごと複製・apply・destroy しづらい
- 標準構成からの差分を昇格・比較しづらい

tesh の役割：

- land: 標準構成
- shrub: 差分パッチ
- env: land + shrubs の適用結果として存在する環境
- output: build された完全な Terraform/OpenTofu 生成物

tesh は Terraform を置き換えない。
Terraform は実行、tesh は編集。

## 2. 設計原則

### 原則1: Terraform の挙動は変えない
- build 後の成果物は通常の .tf
- plan/apply/destroy の意味は Terraform/OpenTofu に委ねる
- state, graph, provider semantics は Terraform 側の責務

### 原則2: 環境差分は branch ではなく env と shrub で扱う
- Git branch は履歴管理
- tesh env は並行環境管理
- shrub は land に対する差分パッチ

### 原則3: 差分は意味単位として読めるべき
- 何を試しているかが shrub 名と内容で分かる
- diff は値差分だけでなく構成差分として表現される

### 原則4: 検証環境は簡単に生やせて簡単に捨てられるべき
- tesh newenv で標準構成から派生環境を作る
- build + apply で即座に環境確認できる
- destroy まで含めてワークフロー化する

### 原則5: 差分は昇格できるべき
- shrub は一時検証にも永続差分にも使える
- 検証済み差分は land へ promote できる

### 原則6: 競合はテキストより手前で見つける
- Git はファイル競合を扱う
- tesh は構成競合を扱う
- 同じ module の同じ属性に対する複数 shrub の上書きは検知対象


## 3. 概念モデル

### 3.1 land
標準構成。すべての env の基底となる。

役割：
- 現行の安定版構成
- 共通部品
- 基本形
- promote の最終着地点

### 3.2 shrub
land に対する差分パッチ。
機能追加、設定変更、削除、実験、一時検証、移行用変更などを表す。

shrub の性質：
- 差分だけ書いてもよい
- land と同じ記法で丸ごと書いてもよい
- 必ずしも永続機能ではない
- promote / move / squash 対象になりうる

type の例：
- experiment
- feature
- migration
- hotfix
- override

### 3.3 env
land + 0個以上の shrubs を適用した結果として存在する環境定義。

### 3.4 output
build によって生成される完成 Terraform/OpenTofu 構成。
人が読める・差分比較できる・破棄再生成できる成果物。

## 4. ディレクトリ構成

```text
project/
  tesh.toml
  land/
    vpc.tf
    ec2.tf
    iam.tf
  shrubs/
    waf/
      shrub.toml
      waf.tf
    cloudfront/
      shrub.toml
      cloudfront.tf
    monitoring/
      shrub.toml
      monitoring.tf
  envs/
    dev/
      env.toml
    stg/
      env.toml
    prod/
      env.toml
  output/
    dev/
    stg/
    prod/
```

## 5. 設定ファイル

### 5.1 tesh.toml

```toml
name = "sample-platform"
terraform_binary = "terraform"
output_dir = "output"

[defaults]
strict = true
state_mode = "remote_only"
```

### 5.2 shrubs/waf/shrub.toml

```toml
name = "waf"
description = "WAF v2 rules for production"
type = "feature"

[metadata]
owner = "gomi"
purpose = "Security hardening"
```

### 5.3 envs/prod/env.toml

```toml
name = "prod"
base = "land"
shrubs = ["waf", "cloudfront", "monitoring"]

[context]
provider = "aws"
account = "production"
region = "ap-northeast-1"
backend = "s3://my-tfstate/prod"

[metadata]
purpose = "Production environment"
```

## 6. マージ仕様

### 6.1 適用順序
1. land
2. env 固有 override
3. shrubs（env.toml に並んだ順）
4. CLI override（将来）

shrubs 順序は明示的。暗黙順禁止。

### 6.2 map/object
deep merge。同じキーがあれば後勝ち。

### 6.3 scalar
後勝ち。

### 6.4 list
replace（v0.1）。append/merge はやらない。複雑化回避のため将来検討。

### 6.5 delete
特殊記法で明示。

```toml
[module.inputs]
timeout = { __delete__ = true }
```

### 6.6 unknown target
存在しない module/resource を shrub が override したら error。
strict=false のとき warning に落とす余地あり。

### 6.7 conflict
同一 env で複数 shrub が同じ属性を上書きした場合：
- デフォルト warning
- strict=true なら error
- tesh explain で最終値の由来を追跡可能にする

## 7. コマンド体系

### 7.1 環境作成
```bash
tesh newenv test-v2 --from prod
```

### 7.2 shrub 作成
```bash
tesh newshrub waf --type feature
```

### 7.3 shrub 追加
```bash
tesh env add-shrub test-v2 waf
```

### 7.4 build
```bash
tesh build dev
```

### 7.5 plan
```bash
tesh plan dev
```

### 7.6 apply
```bash
tesh apply dev
```

### 7.7 diff
```bash
tesh diff prod stg
```

表示レイヤ：
1. shrub 差分
2. config 差分
3. generated tf 差分

### 7.8 explain
```bash
tesh explain dev module.app.inputs.instance_type
```

出力例：
```text
final value: "c7g.xlarge"
source chain:
  land/ec2.tf -> "c7g.large"
  shrubs/apcpuv4/ec2.tf -> "c7g.xlarge"
```

### 7.9 promote
```bash
tesh promote shrub waf --to land
```

### 7.10 doctor
```bash
tesh doctor
```

チェック項目：
- 未解決 override
- conflict
- 不要 shrub
- ttl 切れ実験 shrub
- remote state 方針違反


## 8. 安全性方針

### 8.1 env は実行文脈を持つ
env ごとに最低限以下を明示：
- provider
- account/project
- region
- backend/state location

### 8.2 prod 保護
env.metadata.class = "prod" の場合：
- apply に確認を要求
- destroy は二重確認

### 8.3 remote state only
ローカル state 原則禁止。

### 8.4 output は破棄再生成可能
手編集禁止。生成物に provenance コメントを埋める。

## 9. フェーズ計画

### Phase 1
既存 Terraform を land に寄せる。shrub なしでも価値がある。
env 明示化、output 生成、build、diff、explain。

### Phase 2
差分を shrub と env override に切り出す。
promote/move ワークフロー導入。

### Phase 3
変換可能リソースのマルチクラウド対応。
全リソースの等価性保証はしない。変換可能範囲を明示する。

### Phase 4
抽象 module / multi-cloud 対応の拡充。

## 10. 非目標

tesh がやらないこと：

- Terraform state の独自管理
- Terraform graph の置き換え
- provider semantics の置き換え
- plan/apply ロジックの独自化
- 完全なマルチクラウド等価性保証
- 実行オーケストレーション全般の覇権化

tesh は環境差分編集フレームワークに徹する。

## 11. v0.1 スコープ

### 必須
1. land / shrub / env の厳密定義
2. deterministic build
3. 明文化された merge 仕様
4. newenv
5. build
6. diff
7. explain
8. env context の明示
9. conflict 検出
10. remote state only

### やらない
- マルチクラウド抽象 resource
- list の賢い merge
- 実行 DAG の大規模制御
- hooks
- 独自 DSL の作り込み
- Terraform variable schema 自動生成の深入り

## 12. 定義

tesh は Terraform/OpenTofu のための Git ライクな環境差分フレームワークである。
land を標準構成、shrub を差分パッチ、env を並行運用される実行対象として扱い、検証・比較・昇格を環境単位で可能にする。
Terraform の実行モデルは変えず、構成の編集モデルだけを拡張する。
