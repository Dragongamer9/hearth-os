// Tests only: lets Node (with --experimental-strip-types) import src/*.ts the way the bundler does, without the
// extension ("./classic/model" -> ./classic/model.ts). Usage: node --experimental-strip-types --import ./tests/ts-resolve.mjs --test ...
import { register } from "node:module";
register("data:text/javascript," + encodeURIComponent(`
export async function resolve(spec, ctx, next) {
  try { return await next(spec, ctx); } catch (e) {
    if (!(spec.startsWith("./") || spec.startsWith("../"))) throw e;
    for (const ext of [".ts", ".tsx"]) { try { return await next(spec + ext, ctx); } catch {} }
    throw e;
  }
}`));
