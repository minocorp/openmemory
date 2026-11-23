export type TextPosition = { piece: number; offset: number };

export type QuoteRange = { start: TextPosition; end: TextPosition };

type Letter = { character: string; position: TextPosition };

const whitespace = /\s/u;

export function findQuote(pieces: readonly string[], quote: string): QuoteRange | null {
  const letters = pieces.flatMap(lettersOf);
  const wanted = Array.from(quote.normalize("NFC")).filter(
    (character) => !whitespace.test(character),
  );
  if (wanted.length === 0) {
    return null;
  }
  const haystack = letters.map(({ character }) => character).join("");
  const index = haystack.indexOf(wanted.join(""));
  if (index === -1) {
    return null;
  }
  const first = letters[index];
  const last = letters[index + wanted.length - 1];
  if (!first || !last) {
    return null;
  }
  return {
    start: first.position,
    end: { piece: last.position.piece, offset: last.position.offset + last.character.length },
  };
}

function lettersOf(piece: string, pieceIndex: number): Letter[] {
  const letters: Letter[] = [];
  let offset = 0;
  for (const character of piece) {
    if (!whitespace.test(character)) {
      letters.push({ character, position: { piece: pieceIndex, offset } });
    }
    offset += character.length;
  }
  return letters;
}
