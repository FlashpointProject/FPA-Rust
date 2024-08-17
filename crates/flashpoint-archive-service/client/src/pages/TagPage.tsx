import { useLoaderData } from "react-router-dom";
import { convertKeysToCamelCase, LoaderType } from "../util";
import { Tag } from '@fparchive/flashpoint-archive';
import { StripedHTable } from "../components/StripedTable";

async function getTag(gameId: string): Promise<Tag> {
    const res = await fetch(`/api/tag/${gameId}`);
    if (res.status === 404) {
        throw new Response("Not Found", { status: 404 });
    }
    return convertKeysToCamelCase(await res.json());
}

export async function loader({ params }) {
    const tag = await getTag(params.tagId);
    return tag;
}

export function TagPage() {
    const tag = useLoaderData() as LoaderType<typeof loader>;
    const tableItems: Record<string, any> = {
        'Name': tag.name,
        'Description': tag.description,
        'Aliases': tag.aliases.join('; '),
        'Category': tag.category,
    };

    return (
        <div className="w-full p-5">
            <div className="text-lg italic text-gray-500">Tag: {tag.id}</div>
            <div className="text-2xl my-5 font-bold">{tag.name}</div>
            <div className="text-lg font-bold">Metadata</div>
            <div className="p-5">
                <StripedHTable 
                    items={tableItems}/>
            </div>
        </div>
    );
}