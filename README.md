# terrashrub (tesh)

Terraform/OpenTofu のための環境差分フレームワーク。

Git は履歴を管理する。Terraform は実行する。tesh は環境差分を管理する。

## 概要

tesh は land（標準構成）、shrub（差分パッチ）、env（環境定義）の3層構造で、Terraform の環境差分を可読性よく管理する。

- **land**: 全環境共通のベース構成。生の .tf をそのまま置く。
- **shrub**: 環境ごとに on/off される機能差分。
- **env**: land + shrubs の適用結果として存在する環境。

build すると output/ に通常の Terraform ファイルが生成される。Terraform の挙動は一切変えない。

## ディレクトリ構成

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
  envs/
    dev/
      env.toml
    prod/
      env.toml
  output/
    dev/
    prod/
```

## コマンド

```bash
tesh newenv test-v2 --from prod
tesh newshrub waf --type feature
tesh env add-shrub test-v2 waf
tesh build dev
tesh plan dev
tesh apply dev
tesh diff prod stg
tesh explain dev module.app.inputs.instance_type
tesh promote shrub waf --to land
tesh doctor
```

実装状況: `newenv`、`newshrub`、`env add-shrub`、`build`、`doctor` が動作する。`plan`、`apply`、`diff`、`explain`、`promote` は設計（docs/design.md §7）に沿って未実装で、実行するとその旨を表示する。

`build` は land → env.toml に並んだ順の shrubs の順で .tf を重ね（同名ファイルは後勝ち）、生成ファイルへ由来（provenance）コメントを付けて output/<env>/ へ出力する。出力は決定的で、output/ は毎回再生成される。

## 設計原則

- Terraform の挙動は変えない
- land に生の .tf をそのまま置ける（学習コストゼロ）
- ls するだけで環境の複雑度がわかる
- output は完全な生成物、いつでも破棄再生成できる
- state は S3 等リモートのみ、ローカル禁止

## 詳細設計

[docs/design.md](docs/design.md) を参照。

## License

MIT