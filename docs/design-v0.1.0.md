# terrashrub（tesh）全体設計 v0.1.0

> **出典・位置づけ（2026-11-01 git 取り込み時の注記）**
> - 出典: Notion「terrashrub（tesh）全体設計 v0.1.0」 https://app.notion.com/p/32e6a05ca4d38085876dcef13a8f7bf5?pvs=204 の全文転記（書式のみ Markdown に整えた）。
> - 本書は v0.2 の前身となる全体設計である。現行の正は [design.md](design.md)（v0.2）で、本書と食い違う場合は design.md が優先する。
> - 本書固有の内容として、tesh.toml の全体像、build の処理フロー、「抜けてそうなところ」の洗い出しを含む。

**思想：**

Terraformの環境差分を可読性よく管理するフレームワーク。共通をland、差分をshrub、環境設定をenvsに分離する。lsするだけで環境の複雑度がわかる。outputは完全な生成物で、最終的に普通のterraformがそのまま使える。将来的にマルチクラウド抽象化レイヤーを載せる。

**ディレクトリ構成：**

```txt
project/
├── tesh.toml
├── land/
│   ├── vpc.tf          # Phase 1: 生tf
│   ├── ec2.tf          # Phase 2以降: .teshも置ける
│   └── iam.tf
├── shrub/
│   ├── waf.tf
│   ├── cloudfront.tf
│   └── monitoring.tf
├── envs/
│   ├── dev/            # ファイル少ない = 標準に近い
│   │   └── land.toml
│   ├── stg/
│   │   ├── land.toml
│   │   └── shrubs.toml
│   └── prod/
│       ├── land.toml
│       ├── shrubs.toml
│       ├── waf.toml
│       └── cloudfront.toml
└── output/             # gitignore、完全な生成物
    ├── dev/
    │   ├── config.toml     # マージ済み最終設定
    │   ├── backend.tf
    │   ├── variables.tf
    │   ├── terraform.tfvars
    │   ├── vpc.tf
    │   ├── ec2.tf
    │   └── iam.tf
    ├── stg/
    └── prod/
        ├── config.toml
        ├── backend.tf
        ├── variables.tf
        ├── terraform.tfvars
        ├── vpc.tf
        ├── ec2.tf
        ├── iam.tf
        ├── waf.tf
        └── cloudfront.tf
```

**tesh.toml（プロジェクト共通ベース）：**

```toml
[project]
name = "my-service"

[provider]
cloud = "aws"
region = "ap-northeast-1"
profile = "default"

[backend]
bucket = "my-state-bucket"
region = "ap-northeast-1"
# keyは自動生成: {project.name}/{env}/terraform.tfstate

[land.variables]
env_prefix = "my-service"

[shrubs]
use = []
```

**envs/prod/land.toml（オーバーライド例）：**

```toml
[land.variables]
instance_type = "m5.large"
vpc_cidr = "10.1.0.0/16"
```

**envs/prod/shrubs.toml：**

```toml
[shrubs]
use = ["waf.tf", "cloudfront.tf", "monitoring.tf"]
```

**envs/prod/waf.toml：**

```toml
[shrub.waf.variables]
rule_set = "strict"
```

**マージ順序：**

```txt
tesh.toml（ベース）
  ↓ ディープマージ
envs/{env}/*.toml（オーバーライド、全ファイル読み込み）
  ↓
output/{env}/config.toml（最終形）
```

**コマンド体系：**

```txt
tesh newenv {name}     envs/{name}/ を生成（tesh.tomlベースのテンプレート）
tesh build {env}       land + shrub + env マージ → output/{env}/ にtf生成
tesh plan {env}        build + terraform init + terraform plan
tesh apply {env}       build + terraform init + terraform apply
tesh destroy {env}     output/{env}/ で terraform destroy
tesh diff {env1} {env2}  output同士のconfig.toml差分表示
```

**tesh build \{env\} の処理フロー：**

```txt
1. tesh.toml 読み込み
2. envs/{env}/*.toml 読み込み
3. ディープマージ（envs が tesh.toml を上書き）
4. [backend] 存在チェック（無ければエラー）
5. output/{env}/ をクリーンアップして再生成
6. config.toml 出力（マージ済み最終設定）
7. backend.tf 生成（S3設定、keyは自動: {project}/{env}/terraform.tfstate）
8. provider.tf 生成（cloud, region, profile から）
9. land/ の全tfファイルを output/{env}/ にコピー
10. [shrubs.use] で指定されたshrub/のtfをコピー
11. マージ済みの全variablesから variables.tf を生成（variable宣言）
12. terraform.tfvars を生成（値の代入）
13. 完了
```

**Phase 1（今）：**

land/shrubに生のtfを置く。プロバイダー固定。マージとtf生成。

**Phase 2：**

land/shrubに.teshファイルも置ける。拡張子で分岐。.tfはコピー、.teshは変換。

**Phase 3：**

.teshの抽象リソース定義からプロバイダー別のtfを生成。マルチクラウド対応。envs側でprovider.cloudを上書きすれば同じlandから異なるクラウドのtfが出る。

**設計原則：**

- lsで環境の複雑度がわかる
- config.tomlを開けば環境の全貌がわかる
- diffでconfig.toml同士を比較すれば環境差分がわかる
- outputは完全な生成物、いつでも破棄再生成
- stateはS3のみ、ローカル禁止
- terraformの挙動は一切変えない
- Phase 1の構造がPhase 3を壊さない

**抜けてそうなところ：**

- envs/\*.tomlのファイル名ルール（自由？予約名？）
- land内のtfでvar使う時のvariable宣言の自動生成方法
- shrub同士の依存関係管理
- terraform init の .terraform ディレクトリとlock.hclの扱い
- outputクリーンアップ時にterraform initのキャッシュまで消すか
- tesh diffのフォーマット
- エラーハンドリング（shrubに指定したファイルが存在しない等）
