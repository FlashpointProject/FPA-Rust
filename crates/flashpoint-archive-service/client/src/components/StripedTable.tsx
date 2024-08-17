export type StripedHTableProps = {
    items: Record<string, any>;
}

export function StripedHTable(props: StripedHTableProps) {
    return (
        <table className="table-auto fp-table w-full border-2 border-gray-300 dark:border-gray-600">
            <tbody>
                {Object.entries(props.items).map((item, index) => {
                    const even = index % 2 === 0;
                    const rowClass = even ?
                        'bg-red-100 dark:bg-zinc-800' :
                        'bg-red-50 dark:bg-zinc-700';
                    return (
                        <tr key={index} className={rowClass}>
                            <td className="w-1 whitespace-nowrap p-2">{item[0]}</td>
                            <td className="pr-2">{item[1]}</td>
                        </tr>
                    );
                })}
            </tbody>
        </table>
    );
}