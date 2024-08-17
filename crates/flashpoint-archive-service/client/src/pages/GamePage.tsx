import { Link, useLoaderData } from "react-router-dom";
import { convertKeysToCamelCase, LoaderType } from "../util";
import { Game } from '@fparchive/flashpoint-archive';
import { StripedHTable } from "../components/StripedTable";
import { MultiSelectDropdown } from "../components/Dropdown";

async function getGame(gameId: string): Promise<Game> {
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

export function GamePage() {
    const game = useLoaderData() as LoaderType<typeof loader>;
    const playModes = game.playMode.split(';').map(s => s.trim());

    const tableItems: Record<string, any> = {
        'Library': game.library,
        'Title': game.title,
        'Alternate Titles': game.alternateTitles,
        'Developer': game.developer,
        'Publisher': game.publisher,
        'Series': game.series,
        'Platform': <Link to={`/platform/${game.primaryPlatform}`}>{game.primaryPlatform}</Link>,
        'Date Added': game.dateAdded,
        'Play Mode': <MultiSelectDropdown selected={playModes} locked />,
        'Status': game.status,
        'Source': game.source,
        'Release Date': game.releaseDate,
        'Version': game.version,
        'Original Description': game.originalDescription,
        'Language': game.language,
        'Notes': game.notes,
    };

    const addAppItems: Array<Record<string, any>> = game.addApps ?
        game.addApps.map((addApp) => {
            return {
                'Name': addApp.name,
                'Application Path': addApp.applicationPath,
                'Launch Command': addApp.launchCommand,
                'Auto Run Before': addApp.autoRunBefore,
                'Wait For Exit': addApp.waitForExit,
            };
        }) :
        [];

    const gameDataItems: Array<Record<string, any>> = game.gameData ?
        game.gameData.map((gd) => {
            return {
                'Date Added': gd.dateAdded,
                'SHA256': <div className='font-mono'>{gd.sha256.toUpperCase()}</div>,
                'Size': gd.size,
                'Parameters': gd.parameters,
                'Application Path': gd.applicationPath,
                'Launch Command': gd.launchCommand,
            };
        }) :
        [];

    return (
        <div className="w-full p-5">
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
        </div>
    );
}