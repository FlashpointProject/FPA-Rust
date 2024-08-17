import { useState } from 'react';
import { ImCheckmark, ImCross } from "react-icons/im";

export type MultiSelectDropdownProps = {
    options?: string[];
    selected: string[];
    locked?: boolean;
    onChange?: (selected: string[]) => void;
    placeholder?: string;
};

export function MultiSelectDropdown(props: MultiSelectDropdownProps) {
    const [isOpen, setIsOpen] = useState(false);
    const [searchTerm, setSearchTerm] = useState("");

    const toggleDropdown = () => {
        setIsOpen(!isOpen);
        return false;
    };

    const handleOptionClick = (option: string) => {
        if (props.onChange) {
            if (props.selected.includes(option)) {
                props.onChange(props.selected.filter((item) => item !== option));
            } else {
                props.onChange([...props.selected, option]);
            }
        }
    };

    const handleRemoveSelected = (option: string) => {
        if (props.onChange) {
            props.onChange(props.selected.filter((item) => item !== option));
        }
    };

    const filteredOptions = props.options ? props.options.filter(option =>
        option.toLowerCase().includes(searchTerm.toLowerCase())
    ) : [];

    return (
        <div className="w-full h-full py-1">
            {/* Selected items */}
            <div className="flex flex-wrap">
                {props.selected.map((item) => (
                    <div
                        key={item}
                        className="flex items-center bg-blue-500 dark:bg-blue-700 text-white text-sm rounded-md px-2 py-1 mr-2 mb-2">
                        {item}
                        {!props.locked && (
                            <div
                                className="p-0.5 ml-1 cursor-pointer group"
                                onClick={() => handleRemoveSelected(item)}
                            >
                                <ImCross className="group-hover:text-red-400" />
                            </div>
                        )}
                    </div>
                ))}
            </div>

            {/* Dropdown input */}
            {
                !props.locked && (
                    <div className="relative w-full">
                        <div className="w-full border-2 border-gray-300 bg-white dark:bg-gray-800 dark:border-gray-600 rounded-md  p-2 flex items-center">
                            <input
                                className="w-full bg-white text-black placeholder-gray-500 dark:bg-gray-800 dark:text-white dark:placeholder-gray-400 focus:outline-none"
                                type="text"
                                placeholder={props.placeholder}
                                value={searchTerm}
                                onChange={(event) => setSearchTerm(event.target.value)}
                                onClick={toggleDropdown}
                            />
                            <div
                                className={`w-5 h-5 ml-2 transform ${isOpen ? 'rotate-180' : ''} cursor-pointer`}
                                onClick={toggleDropdown}
                            />
                        </div>

                        {/* Dropdown menu */}
                        {isOpen && (
                            <div className="absolute z-10 w-full bg-white dark:bg-gray-800 border-2 border-gray-300 dark:border-gray-600 rounded-md mt-1 max-h-40 overflow-y-auto">
                                {filteredOptions.length > 0 ? (
                                    filteredOptions.map((option) => (
                                        <div
                                            key={option}
                                            className={`p-2 cursor-pointer flex flex-row items-center hover:bg-blue-100 dark:hover:bg-gray-700`}
                                            onClick={() => handleOptionClick(option)}>
                                            <div className='flex-shrink-0 w-auto mr-1 h-full'>
                                                {props.selected.includes(option) && (
                                                    <ImCheckmark className='w-full h-full' />
                                                )}
                                            </div>
                                            <div className='flex-grow'>
                                                {option}
                                            </div>
                                        </div>
                                    ))
                                ) : (
                                    <div className="p-2 text-gray-500 dark:text-gray-400">No options found</div>
                                )}
                            </div>
                        )}
                    </div>
                )
            }
        </div >
    );
}