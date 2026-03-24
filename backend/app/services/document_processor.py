"""Service de découpage (chunking) de documents texte."""
from __future__ import annotations

import re


def chunk_text(
    text: str,
    chunk_size: int = 1500,
    overlap: int = 150,
) -> list[tuple[str, int]]:
    """Découpe un texte en chunks avec chevauchement.

    Returns:
        Liste de tuples (contenu_chunk, nb_tokens_approximatif)
    """
    # Nettoyage basique
    text = text.strip()
    if not text:
        return []

    # Séparateurs par ordre de priorité
    separators = ["\n\n\n", "\n\n", "\n", ". ", " "]
    chunks: list[tuple[str, int]] = []

    paragraphs = _split_by_separators(text, separators, chunk_size)

    buffer = ""
    for para in paragraphs:
        if not para.strip():
            continue
        candidate = (buffer + "\n\n" + para).strip() if buffer else para.strip()
        tokens = _estimate_tokens(candidate)

        if tokens <= chunk_size:
            buffer = candidate
        else:
            if buffer:
                chunks.append((buffer, _estimate_tokens(buffer)))
            # Si le paragraphe seul dépasse chunk_size, le forcer
            if _estimate_tokens(para) > chunk_size:
                sub_chunks = _force_split(para, chunk_size)
                chunks.extend(sub_chunks)
                buffer = ""
            else:
                buffer = para.strip()

    if buffer:
        chunks.append((buffer, _estimate_tokens(buffer)))

    # Applique le chevauchement
    if overlap > 0 and len(chunks) > 1:
        chunks = _apply_overlap(chunks, overlap)

    return chunks


def _split_by_separators(text: str, separators: list[str], max_tokens: int) -> list[str]:
    """Tente de couper aux séparateurs naturels."""
    for sep in separators:
        parts = text.split(sep)
        if all(_estimate_tokens(p) <= max_tokens for p in parts):
            return parts
    return [text]


def _force_split(text: str, chunk_size: int) -> list[tuple[str, int]]:
    """Coupe brutalement un texte trop long par mots."""
    words = text.split()
    chunks = []
    buffer_words: list[str] = []
    current_tokens = 0

    for word in words:
        wt = max(1, len(word) // 4)
        if current_tokens + wt > chunk_size and buffer_words:
            chunk = " ".join(buffer_words)
            chunks.append((chunk, _estimate_tokens(chunk)))
            buffer_words = []
            current_tokens = 0
        buffer_words.append(word)
        current_tokens += wt

    if buffer_words:
        chunk = " ".join(buffer_words)
        chunks.append((chunk, _estimate_tokens(chunk)))

    return chunks


def _apply_overlap(
    chunks: list[tuple[str, int]],
    overlap: int,
) -> list[tuple[str, int]]:
    """Ajoute un chevauchement entre chunks consécutifs."""
    result: list[tuple[str, int]] = [chunks[0]]
    for i in range(1, len(chunks)):
        prev_text = chunks[i - 1][0]
        curr_text = chunks[i][0]
        # Prend les derniers mots du chunk précédent comme préfixe
        prev_words = prev_text.split()
        overlap_words = prev_words[-overlap:] if len(prev_words) > overlap else prev_words
        prefix = " ".join(overlap_words)
        merged = (prefix + "\n" + curr_text).strip()
        result.append((merged, _estimate_tokens(merged)))
    return result


def _estimate_tokens(text: str) -> int:
    """Estimation rapide du nombre de tokens (≈ 4 caractères par token)."""
    return max(1, len(text) // 4)


def process_file(path: str, content: str) -> list[tuple[str, int]]:
    """Point d'entrée principal pour le traitement d'un fichier."""
    # Détecte si c'est du code source → meilleur découpage par blocs
    code_extensions = {".py", ".rs", ".js", ".ts", ".go", ".java", ".c", ".cpp", ".sh"}
    import os
    ext = os.path.splitext(path)[1].lower()

    if ext in code_extensions:
        return _chunk_code(content)

    return chunk_text(content)


def _chunk_code(content: str, chunk_size: int = 1500) -> list[tuple[str, int]]:
    """Découpage intelligent pour du code source (par fonctions/classes)."""
    # Sépare sur les blocs de fonctions/classes (lignes vides + définition)
    blocks = re.split(r"\n(?=\s*(?:def |class |fn |function |func |async def |pub fn |impl ))", content)
    chunks: list[tuple[str, int]] = []
    buffer = ""

    for block in blocks:
        candidate = (buffer + "\n\n" + block).strip() if buffer else block.strip()
        tokens = _estimate_tokens(candidate)

        if tokens <= chunk_size:
            buffer = candidate
        else:
            if buffer:
                chunks.append((buffer, _estimate_tokens(buffer)))
            if _estimate_tokens(block) > chunk_size:
                chunks.extend(_force_split(block, chunk_size))
                buffer = ""
            else:
                buffer = block.strip()

    if buffer:
        chunks.append((buffer, _estimate_tokens(buffer)))

    return chunks if chunks else chunk_text(content)
