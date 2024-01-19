import * as fs from 'fs';

const addition = `
export type TagVec = string[];
`;

const main = async () => {
    await fs.promises.appendFile('index.d.ts', addition, { encoding: 'utf-8' });
    console.log("build.mjs: Modified generated typings");
};

main();