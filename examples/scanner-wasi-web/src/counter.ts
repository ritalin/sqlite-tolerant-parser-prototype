// 最初、`https://github.com/jprendes/rust-wasm/blob/main/wasip2.md`をみて、明示的にinstantiateが必要と思って書いたコードの残骸
//
// import {instantiate} from 'pkg/scanner/scanner'
// import {type TokenItem} from 'pkg/scanner/interfaces/ritalin-scanner-types'

// import pkgScanner from 'pkg/scanner/scanner?url'
// // import * as imports from '@bytecodealliance/preview2-shim'
// // @ts-ignore
// import imports from "https://cdn.jsdelivr.net/gh/bytecodealliance/jco@a72d4b38/packages/preview2-shim/lib/browser/wasip2.js"

// const pkgUrl = new URL(pkgScanner, import.meta.url)
// const { scanner: scanners } = await instantiate(
//   async (url) => fetch(new URL(url, pkgUrl)).then(WebAssembly.compileStreaming),
//   { ...imports }
// )

// export function setupCounter(_element: HTMLButtonElement) {

//   const source = "SELECT * FROM foo a;"
//   const scanner = scanners.Scanner.create(source, 0)

//   let lookahead
//   while ((lookahead = scanner.shift()) != undefined) {
//     if (lookahead.leading) {
//       for (const item of lookahead.leading) {
//         printToken(item, "Leading")
//       }
//     }

//     printToken(lookahead.main, "Main")

//     if (lookahead.trailing) {
//       for (const item of lookahead.trailing) {
//         printToken(item, "Trailing")
//       }
//     }
//   }
// }

import {scanner} from 'pkg/scanner/scanner'
import {type TokenItem} from 'pkg/scanner/interfaces/ritalin-scanner-types'

export function setupCounter(_element: HTMLButtonElement) {
  const source = "SELECT * FROM foo a;"
  console.log(`source: "${source}"`)
  console.log("----------------------------------------")

  const s = scanner.Scanner.create(source, 0)

  let lookahead
  while ((lookahead = s.shift()) != undefined) {
    if (lookahead.leading) {
      for (const item of lookahead.leading) {
        printToken(item, "Leading")
      }
    }

    printToken(lookahead.main, "Main")

    if (lookahead.trailing) {
      for (const item of lookahead.trailing) {
        printToken(item, "Trailing")
      }
    }
  }
}


function printToken(item: TokenItem, context: string) {
  console.log(`[${context}] kind: { id: ${item.kind.id}, name: ${item.kind.text} }, offset: ${item.offset}, len: ${item.len}, value: ${item.value}`)
}