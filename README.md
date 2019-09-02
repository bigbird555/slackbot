# slackbot

[![Build Status][travis-image]][travis-url]
[![Coverage Status][codecov-image]][codecov-url]

[![Coverage Graph][codecov-graph]][codecov-url]

[travis-url]: https://travis-ci.org/tsg-ut/slackbot
[travis-image]: https://travis-ci.org/tsg-ut/slackbot.svg?branch=master
[codecov-url]: https://codecov.io/gh/tsg-ut/slackbot
[codecov-image]: https://codecov.io/gh/tsg-ut/slackbot/branch/master/graph/badge.svg
[codecov-graph]: https://codecov.io/gh/tsg-ut/slackbot/branch/master/graphs/tree.svg?width=888&height=150

TSGのSlackで動くSlackbotたち

自分がOWNERのコードの変更は直接masterにpushして構いません。 ([CODEOWNERS](CODEOWNERS)参照)

## 環境構築

### Prerequisites

* Node.js Latest

### セットアップ

```sh
cd /path/to/slackbot
npm install
cp .env.example .env
# .envをいい感じに編集する
```

`CHANNEL_XXX`系は全部自分宛のDMを指定するのがオススメ。

`SLACK_TOKEN` は [@tsgbot の OAuth & Permissions](https://api.slack.com/apps/ADMCWEP5X/oauth) から必要な権限のみに絞ったTokenを発行するのをオススメ。アクセスできない場合は管理者権限がありそうな人に聞いてください。

大抵必要な設定項目

* `HAKATASHI_TOKEN`: 自分のUser token
* `SLACK_TOKEN`: 自分のBot token
* `SIGNING_SECRET`: 手元でテストする分には適当な文字列でよい
* `USER_TSGBOT`: @tsgbotのユーザー名

必要なスコープ

* `channels:history`
* `channels:write`
* `chat:write:bot`
* `chat:write:user`
* `incoming-webhook`
* `bot`
* `commands`
* `users:read`

`IMAGEBIN_KEY`はshogiを開発する時以外は必要ない。必要な場合は https://imagebin.ca/tools.php からAPIキーを取得。

#### shogiのセットアップ

[nine-grids-shogi-analyzer](https://github.com/hakatashi/nine-grids-shogi-analyzer)を実行したら生成される`test.sqlite3`を`slackbot/shogi/boards/test.sqlite3`に配置する。

### 実行

```sh
npm run dev
```

## デプロイ

自動デプロイです。[deploy](deploy)参照。

# Licenses

このリポジトリでは以下のライブラリを使用しています。

* Shogi Resource by muchonovski is licensed under a Creative Commons 表示-非営利 2.1 日本 License.
