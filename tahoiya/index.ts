import {promises as fs, constants} from 'fs';
// @ts-ignore
import download from 'download';
import path from 'path';
import {RTMClient, WebClient, SectionBlock} from '@slack/client';
import sql from 'sql-template-strings';
import sqlite from 'sqlite';
import {Mutex} from 'async-mutex';
import {sampleSize, chunk} from 'lodash';
// @ts-ignore
import {stripIndent} from 'common-tags';
import {Deferred} from '../lib/utils';
import {getMemberName} from '../lib/slackUtils';
import {Message} from '../lib/slackTypes';
// @ts-ignore
import logger from '../lib/logger';
import {
	getPageTitle,
	getWordUrl,
	getIconUrl,
	getTimeLink,
	getMeaning,
	getCandidateWords,
	normalizeMeaning,
} from './lib';

interface SlackInterface {
	rtmClient: RTMClient,
	webClient: WebClient,
	messageClient: any,
}

interface Game {
	time: number,
	duration: number,
	word: string,
}

interface State {
	games: Game[],
}

const loadDeferred = new Deferred();

const mutex = new Mutex();

const wordsVersion = '201907260000';

class Tahoiya {
	tsgRtm: RTMClient;
	tsgSlack: WebClient;
	kmcRtm: RTMClient;
	kmcSlack: WebClient;
	slackInteractions: any;
	state: State;
	words: string[];

	constructor({tsgRtm, tsgSlack, kmcRtm, kmcSlack, slackInteractions}: {tsgRtm: RTMClient, tsgSlack: WebClient, kmcRtm: RTMClient, kmcSlack: WebClient, slackInteractions: any}) {
		this.tsgRtm = tsgRtm;
		this.tsgSlack = tsgSlack;
		this.kmcRtm = kmcRtm;
		this.kmcSlack = kmcSlack;
		this.slackInteractions = slackInteractions;

		this.state = {
			games: [],
		};
	}

	// TODO: lock
	async initialize() {
		if (loadDeferred.isResolved) {
			return loadDeferred.promise;
		}

		for (const file of ['words.txt', 'words.sqlite3']) {
			const filename = file.replace(/\./, `.${wordsVersion}.`);
			const filePath = path.resolve(__dirname, filename);
			const exists = await fs.access(filePath, constants.F_OK).then(() => true).catch(() => false);
			if (!exists) {
				await download(`https://s3-ap-northeast-1.amazonaws.com/hakata-public/slackbot/tahoiya/${file}`, __dirname, {
					filename,
				});
			}
		}

		const wordsBuffer = await fs.readFile(path.resolve(__dirname, `words.${wordsVersion}.txt`));
		this.words = wordsBuffer.toString().split('\n').filter((l) => l.length > 0);

		const statePath = path.resolve(__dirname, 'state.json');
		const stateExists = await fs.access(statePath, constants.F_OK).then(() => true).catch(() => false);
		if (stateExists) {
			const stateData = await fs.readFile(statePath);
			Object.assign(this.state, JSON.parse(stateData.toString()));
		}

		this.slackInteractions.action({
			type: 'button',
			blockId: /^tahoiya_add_meaning/,
		}, (payload: any, respond: any) => {
			const [action] = payload.actions;

			this.tsgSlack.dialog.open({
				trigger_id: payload.trigger_id,
				dialog: {
					callback_id: 'tahoiya_add_meaning_dialog',
					title: `「${action.value}」の意味を考えてね！`,
					submit_label: '登録する',
					notify_on_cancel: true,
					state: action.value,
					elements: [
						{
							type: 'text',
							label: `「${action.value}」の意味`,
							name: 'meaning',
							min_length: 3,
							value: 'ほげぷがぴよぴよ',
							hint: '後から変更できます',
						},
						{
							type: 'textarea',
							label: 'コメント',
							name: 'comment',
							optional: true,
							value: 'ほげぷがぴよぴよ',
							hint: '後から変更できます',
						},
					],
				},
			});
		});

		this.slackInteractions.action({
			type: 'dialog_submission',
			callbackId: 'tahoiya_add_meaning_dialog',
		}, (payload: any, respond: any) => {
			console.log(payload);
		});

		this.slackInteractions.action({
			type: 'button',
			blockId: /^start_tahoiya/,
		}, (payload: any, respond: any) => {
			const [action] = payload.actions;
			mutex.runExclusive(async () => {
				this.startTahoiya(action.value, respond);
			});
		});

		loadDeferred.resolve();
	}

	async generateCandidates() {
		if (this.state.games.length > 2) {
			throw new Error('たほいやを同時に3つ以上開催することはできないよ:imp:');
		}

		const candidates = sampleSize(this.words, 20);

		this.tsgSlack.chat.postMessage({
			channel: process.env.CHANNEL_SANDBOX,
			username: 'tahoiya',
			icon_emoji: ':open_book:',
			text: '',
			blocks: [
				{
					type: 'section',
					text: {
						type: 'mrkdwn',
						text: stripIndent`
							たのしい＊たほいや＊を始めるよ〜👏👏👏
							下のリストの中からお題にする単語を選んでクリックしてね:wink:
						`,
					},
				},
				...(chunk(candidates, 5).map((candidateGroup, index) => ({
					type: 'actions',
					block_id: `start_tahoiya_${index}`,
					elements: candidateGroup.map((candidate) => ({
						type: 'button',
						text: {
							type: 'plain_text',
							text: candidate,
						},
						value: candidate,
						confirm: {
							text: {
								type: 'plain_text',
								text: `お題を「${candidate}」にセットしますか?`,
							},
							confirm: {
								type: 'plain_text',
								text: 'いいよ',
							},
							deny: {
								type: 'plain_text',
								text: 'だめ',
							},
						},
					})),
				}))),
			],
		});
	}

	async startTahoiya(word: string, respond: any) {
		if (this.state.games.length > 2) {
			respond({
				text: 'たほいやを同時に3つ以上開催することはできないよ:imp:',
				response_type: 'ephemeral',
				replace_original: false,
			});
			return;
		}

		const now = Date.now();
		const game = {
			time: now,
			duration: 5 * 60 * 1000,
			word,
		};

		this.setState({
			games: this.state.games.concat([game]),
		});

		this.tsgSlack.chat.postMessage({
			channel: process.env.CHANNEL_SANDBOX,
			username: 'tahoiya',
			icon_emoji: ':open_book:',
			text: '',
			blocks: [
				{
					type: 'section',
					text: {
						type: 'mrkdwn',
						text: stripIndent`
							お題を＊「${word}」＊に設定したよ:v:
							参加者は5分以内にこの単語の意味を考えて <@${process.env.USER_TSGBOT}> にDMしてね:relaxed:
							終了予定時刻: ${getTimeLink(game.time + game.duration)}
						`,
					},
				},
				{type: 'divider'},
				...this.getGameBlocks(),
			],
		});
	}

	showStatus() {
		return this.tsgSlack.chat.postMessage({
			channel: process.env.CHANNEL_SANDBOX,
			username: 'tahoiya',
			icon_emoji: ':open_book:',
			text: '',
			blocks: [
				...this.getGameBlocks(),
			],
		});
	}

	async setState(object: Partial<State>) {
		Object.assign(this.state, object);
		const statePath = path.resolve(__dirname, 'state.json');
		await fs.writeFile(statePath, JSON.stringify(this.state));
	}

	getMention(user: string) {
		if (user === 'tahoiyabot-01') {
			return 'たほいやAIくん1号 (仮)';
		}

		if (user === 'tahoiyabot-02') {
			return 'たほいやAIくん2号 (仮)';
		}

		return `<@${user}>`;
	};

	getGameBlocks(): SectionBlock[] {
		if (this.state.games.length === 0) {
			return [{
				type: 'section',
				text: {
					type: 'mrkdwn',
					text: '現在行われているたほいやはありません:cry:'
				},
			}];
		}

		return this.state.games.map((game, index) => ({
			type: 'section',
			block_id: `tahoiya_add_meaning_${index}`,
			text: {
				type: 'mrkdwn',
				text: stripIndent`
					🍣 お題＊「${game.word}」＊
					終了予定時刻: ${getTimeLink(game.time + game.duration)}
				`,
			},
			accessory: {
				type: 'button',
				text: {
					type: 'plain_text',
					text: '登録する',
				},
				value: game.word,
			},
		}))
	}
}

module.exports = async ({rtmClient: tsgRtm, webClient: tsgSlack, messageClient: slackInteractions}: SlackInterface) => {
	const tokensDb = await sqlite.open(path.join(__dirname, '..', 'tokens.sqlite3'));
	const kmcToken = await tokensDb.get(sql`SELECT * FROM tokens WHERE team_id = ${process.env.KMC_TEAM_ID}`);
	const kmcSlack = kmcToken === undefined ? null : new WebClient(kmcToken.bot_access_token);
	const kmcRtm = kmcToken === undefined ? null : new RTMClient(kmcToken.bot_access_token);

	const {team: tsgTeam}: any = await tsgSlack.team.info();

	const tahoiya = new Tahoiya({tsgSlack, tsgRtm, kmcSlack, kmcRtm, slackInteractions});
	await tahoiya.initialize();

	const onMessage = (message: Message, team: string) => {
		if (!message.text || message.subtype !== undefined) {
			return;
		}
		

		const text = message.text.trim();

		if (text === 'たほいや') {
			mutex.runExclusive(async () => ( 
				tahoiya.generateCandidates().catch((error) => {
					error.message;
				})
			));
		}

		if (text === 'たほいや 状況') {
			mutex.runExclusive(async () => ( 
				tahoiya.showStatus().catch((error) => {
					error.message;
				})
			));
		}
	};

	tsgRtm.on('message', (event) => {
		onMessage(event, 'TSG');
	});

	if (kmcToken === undefined) {
		logger.info('Disabling KMC tahoiya because token is not found');
	} else {
		kmcRtm.on('message', (event) => {
			onMessage(event, 'KMC');
		});
	}
};
