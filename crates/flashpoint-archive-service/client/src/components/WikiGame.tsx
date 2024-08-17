import { Game } from "@fparchive/flashpoint-archive"
import { Markdown } from "./Markdown"

export type WikiGameProps = {
    game: Game,
    content: JSX.Element,
}

export function WikiGame(props: WikiGameProps) {
    const { content, game } = props;

    return (
        <div className="w-full h-full">
            <div className="border-b-2 border-gray-300 dark:border-gray-600">
                <div className="text-lg italic text-gray-500">{game.id}</div>
                <div className="text-2xl my-5 font-bold">{game.title}</div>
            </div>
            <div className="flex flex-row">
                <div className="flex-grow">
                    {content}
                </div>
                <div className="flex-shrink-0">
                    <div className="mt-2 p-2 border-2 bg-red-100 dark:bg-zinc-800 border-gray-300 dark:border-gray-600 max-w-56">
                        <img src={`https://infinity.unstable.life/images/Logos/${game.id.substring(0, 2)}/${game.id.substring(2, 4)}/${game.id}.png`} />
                        <div className="font-bold">Developer</div>
                        {game.developer.split(';').map(s =>
                            <div className="ml-1">
                                {s.trim()}
                            </div>
                        )}
                        <div className="font-bold">Publisher</div>
                        {game.publisher.split(';').map(s =>
                            <div className="ml-1">
                                {s.trim()}
                            </div>
                        )}
                        <div className="font-bold">Series</div>
                        <div className="ml-1">
                            {game.series}
                        </div>
                    </div>
                </div>
            </div>

        </div>
    )
}