import { Link, useLoaderData } from "react-router-dom";
import { convertKeysToCamelCase, convertKeysToSnakeCase, LoaderType } from "../util";
import { AdditionalApp, Game, GameData } from '@fparchive/flashpoint-archive';
import { StripedHTable } from "../components/StripedTable";
import { useState } from "react";
import { RowEditorInput, RowEditorTextArea } from "../components/RowEditors";
import { MultiSelectDropdown } from "../components/Dropdown";
import { Markdown, MarkdownEditor } from "../components/Markdown";
import { WikiGame } from "../components/WikiGame";
import { MDXEditor } from '@mdxeditor/editor';

async function getGame(gameId: string): Promise<Game> {
    const profileRes = await fetch('/api/profile'); // Check for user perms
    if (profileRes.status !== 200) {
        throw new Response("Unauthorized", { status: 400 });
    }

    const res = await fetch(`/api/game/${gameId}`);
    if (res.status === 404) {
        throw new Response("Not Found", { status: 404 });
    }
    return convertKeysToCamelCase(await res.json());
}

export async function loader({ params }) {
    const game = await getGame(params.gameId);
    return game;
}

enum ActiveTab {
    DATA,
    WIKI,
}

export function EditGamePage() {
    const [game, setGame] = useState(useLoaderData() as LoaderType<typeof loader>);
    const [activeTab, setActiveTab] = useState(ActiveTab.DATA);
    const [wikiData, setWikiData] = useState("");

    const onSaveGame = () => {
        fetch(`/api/game/${game.id}`, {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json'
            },
            body: JSON.stringify(convertKeysToSnakeCase(game)),
        })
            .then(() => {
                alert('saved');
            })
            .catch((err) => {
                alert(`error: ${err}`);
            })
    };

    const editorInputFactory = (key: keyof Game) => <RowEditorInput value={game[key] as string} onChange={(value) => setGame({ ...game, [key]: value })} />;
    const editorTextAreaFactory = (key: keyof Game) => <RowEditorTextArea value={game[key] as string} onChange={(value) => setGame({ ...game, [key]: value })} />;
    const [playModes, setPlayModes] = useState(game.playMode.split(';').map(s => s.trim()));
    const onChangePlayModes = (newPlayModes: string[]) => {
        setPlayModes(newPlayModes)
    }

    const tableItems: Record<string, any> = {
        'Library': game.library,
        'Title': editorInputFactory('title'),
        'Alternate Titles': editorInputFactory('alternateTitles'),
        'Developer': editorInputFactory('developer'),
        'Publisher': editorInputFactory('publisher'),
        'Series': editorInputFactory('series'),
        'Platform': <Link to={`/platform/${game.primaryPlatform}`}>{game.primaryPlatform}</Link>,
        'Date Added': game.dateAdded,
        'Play Mode': <MultiSelectDropdown
            options={['Single Player', 'Multiplayer', 'Cooperative']}
            selected={playModes}
            onChange={onChangePlayModes}
            placeholder="Select Play Modes..."
        />,
        'Status': editorInputFactory('status'),
        'Source': editorInputFactory('source'),
        'Release Date': game.releaseDate,
        'Version': editorInputFactory('version'),
        'Original Description': editorTextAreaFactory('originalDescription'),
        'Language': editorInputFactory('language'),
        'Notes': editorTextAreaFactory('notes'),
    };

    const addAppItems: Array<Record<string, any>> = game.addApps ?
        game.addApps.map((addApp, index) => {
            const editorInputFactory = (key: keyof AdditionalApp) => <RowEditorInput value={addApp[key] as string} onChange={(value) => {
                const newGame = {
                    ...game,
                    addApps: [
                        ...(game.addApps as AdditionalApp[])
                    ]
                };
                Object.assign(newGame.addApps[index], { [key]: value });

                setGame(newGame);
            }} />;

            return {
                'Name': editorInputFactory('name'),
                'Application Path': editorInputFactory('applicationPath'),
                'Launch Command': editorInputFactory('launchCommand'),
                'Auto Run Before': addApp.autoRunBefore,
                'Wait For Exit': addApp.waitForExit,
            };
        }) :
        [];

    const gameDataItems: Array<Record<string, any>> = game.gameData ?
        game.gameData.map((gd, index) => {
            const editorInputFactory = (key: keyof GameData) => <RowEditorInput value={gd[key] as string} onChange={(value) => {
                const newGame = {
                    ...game,
                    gameData: [
                        ...(game.gameData as GameData[])
                    ]
                };
                Object.assign(newGame.gameData[index], { [key]: value });

                setGame(newGame);
            }} />;

            return {
                'Date Added': gd.dateAdded,
                'SHA256': <div className='font-mono'>{gd.sha256.toUpperCase()}</div>,
                'Size': gd.size,
                'Parameters': editorInputFactory('parameters'),
                'Application Path': editorInputFactory('applicationPath'),
                'Launch Command': editorInputFactory('launchCommand'),
            };
        }) :
        [];

    return (
        <div className="w-full p-5">
            {/* Tab Selection Row */}
            <div className="flex border-b border-gray-500 mb-4">
                <button
                    className={`px-4 py-2 text-sm font-medium ${activeTab === ActiveTab.DATA ? 'border-b-2 border-blue-500 text-blue-500' : 'text-gray-400'
                        }`}
                    onClick={() => setActiveTab(ActiveTab.DATA)}
                >
                    Data
                </button>
                <button
                    className={`ml-2 px-4 py-2 text-sm font-medium ${activeTab === ActiveTab.WIKI ? 'border-b-2 border-blue-500 text-blue-500' : 'text-gray-400'
                        }`}
                    onClick={() => setActiveTab(ActiveTab.WIKI)}
                >
                    Wiki
                </button>
            </div>
            {activeTab === ActiveTab.WIKI && (
                <>
                    <WikiGame
                        game={game}
                        content={
                            <MarkdownEditor
                                value={wikiData}
                                onChange={setWikiData}
                                rows={20}
                                innerProps={{
                                    'game': game,
                                }} />
                        }
                    />
                </>
            )}
            {activeTab === ActiveTab.DATA && (
                <>
                    <div className="text-lg italic text-gray-500">{game.id}</div>
                    <div className="text-2xl my-5 font-bold">{game.title}</div>
                    <div className="text-lg font-bold">Metadata</div>
                    <div className="p-5">
                        <StripedHTable
                            items={tableItems} />
                    </div>
                    {addAppItems.length > 0 ? (
                        <div>
                            <div className="text-lg font-bold">Additional Apps</div>
                            {addAppItems.map((items, index) => (
                                <div key={index} className='p-5'>
                                    <StripedHTable
                                        items={items} />
                                </div>
                            ))}
                        </div>
                    ) : undefined}
                    <button className="bg-blue-500 text-white border-blue-600 rounded p-2 font-semibold mb-2"
                        onClick={onSaveGame}>
                        Save Changes
                    </button>
                    {gameDataItems.length > 0 ? (
                        <div>
                            <div className="text-lg font-bold">Game Data</div>
                            {gameDataItems.map((items, index) => (
                                <div key={index} className='p-5'>
                                    <StripedHTable
                                        items={items} />
                                </div>
                            ))}
                        </div>
                    ) : undefined}
                </>
            )}
        </div>
    );
}

