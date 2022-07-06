const rLineOfSpaces = /^\s+$/m;
const rSurroundingLinefeed = /^\n|\n$/;
// eslint-disable-next-line no-control-regex
const rUnprintable = /[\x00-\x09\x0b-\x1f\x7f-\x9f]/;
// eslint-disable-next-line no-control-regex
const rEscapees = /[\x00-\x09\x0b-\x1f\x7f-\x9f&<>]| $/gm;
const rLineOfBackticks = /^\s*```\s*$/m;
const rNewLine = /^/gm;

export default function codeToMarkdown(code: string, language: string | undefined): string {
  if (code === '') {
    return '<pre><code></code></pre>';
  } if (!rLineOfBackticks.test(code) && !rUnprintable.test(code)) {
    return `\`\`\`${language ?? ''}\n${code}\n\`\`\`\``;
  } if (rLineOfSpaces.test(code) || rSurroundingLinefeed.test(code) || rUnprintable.test(code)) {
    return `<pre><code>${code.replace(rEscapees, character => {
      switch (character) {
        case '\0':
          return '';
        case '<':
          return '&lt;';
        case '>':
          return '&gt;';
        case '&':
          return '&amp;';
        default:
          return `&#${character.charCodeAt(0)};`;
      }
    })}\n</code></pre>`;
  }

  return `<!-- language: lang-${language ?? ''} -->\n${
    code.replace(rNewLine, '    ')}`;
}
