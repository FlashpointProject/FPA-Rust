import * as esbuild from 'esbuild';
import * as fs from 'fs';


const config = {
  entryPoints: ['src/index.tsx'],
  bundle: true,
  minify: false,
  format: 'cjs',
  sourcemap: true,
  outfile: '../static/bundle.js',
};

async function build() {
  await fs.promises.copyFile('./node_modules/@wooorm/starry-night/style/both.css', '../static/highlighting.css');
  esbuild.build(config);
}

async function watch() {
  await fs.promises.copyFile('./node_modules/@wooorm/starry-night/style/both.css', '../static/highlighting.css');
  const context = await esbuild.context(config);
  context.watch();
}

if (process.argv[process.argv.length - 1] === '--watch') {
  watch();
} else {
  build();
}
