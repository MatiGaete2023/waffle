"""E-X1: guardia de solo lectura de RUS y SITFA.

Falla (código 1) si el código de producción de `app/src` o las extensiones del Descargador mencionan los controles de
creación o edición de observaciones de la bitácora de RUS, o un envío de Outlook (`.Send(`). Los nombres salen del
inspector integral de la bitácora (informe técnico RUS/SITFA, apéndice B). Flow solo lee y deja borradores.

Uso (desde la raíz): python tools/verificar_solo_lectura.py
"""

import re
import sys
from pathlib import Path

RAIZ = Path(__file__).resolve().parents[1]
CARPETAS = [RAIZ / 'app' / 'src', RAIZ / 'descargador' / 'extension', RAIZ / 'descargador_lite', RAIZ / 'installer']
SUFIJOS = {'.rs', '.js', '.html', '.json', '.ps1'}
PROHIBIDOS = {
    'control de creación o edición de observaciones de RUS': re.compile(
        r'COD_Estado_Detalle(?:_Nuevo)?|id_Obs_centro|GLS_ObsCentro(?:_Nueva)?|GLS_ObsTribunal|GLS_Observacion|'
        r'\bchkBitacora(?:Edit)?\b|\bTxt_Cont2\b|GLS_ObsNoEdit|GLS_ObsTribNoEdit',
        re.IGNORECASE,
    ),
    'envío de correo (solo se guardan borradores)': re.compile(r'\.Send\s*\(|(?i:\.send\s*\(\s*\))'),
}

_CFG_TEST_LINEA = re.compile(r'^\s*#\[cfg\(\s*test\s*\)\]\s*(?://.*)?$')
_ATRIBUTO_O_VACIO = re.compile(r'^\s*(?:#\[.*\]\s*)?(?://.*)?$')


def _consumir_bloque_test(lineas: list[str], inicio: int) -> int:
    """Devuelve el índice de la primera línea posterior al ítem anotado con `#[cfg(test)]`."""
    i = inicio + 1
    n = len(lineas)
    while i < n and _ATRIBUTO_O_VACIO.match(lineas[i]):
        i += 1
    profundidad = 0
    vio_llave = False
    en_bloque_comentario = 0
    en_cadena = False
    raw_hashes: int | None = None
    while i < n:
        linea = lineas[i]
        j = 0
        m = len(linea)
        while j < m:
            ch = linea[j]
            nxt = linea[j + 1] if j + 1 < m else ''
            if en_bloque_comentario > 0:
                if ch == '/' and nxt == '*':
                    en_bloque_comentario += 1
                    j += 2
                elif ch == '*' and nxt == '/':
                    en_bloque_comentario -= 1
                    j += 2
                else:
                    j += 1
                continue
            if raw_hashes is not None:
                if ch == '"':
                    fin = '"' + '#' * raw_hashes
                    if linea.startswith(fin, j):
                        j += len(fin)
                        raw_hashes = None
                        continue
                j += 1
                continue
            if en_cadena:
                if ch == '\\':
                    j += 2
                    continue
                if ch == '"':
                    en_cadena = False
                j += 1
                continue
            if ch == '/' and nxt == '/':
                break
            if ch == '/' and nxt == '*':
                en_bloque_comentario += 1
                j += 2
                continue
            if ch == 'r' and (nxt == '"' or nxt == '#'):
                k = j + 1
                while k < m and linea[k] == '#':
                    k += 1
                if k < m and linea[k] == '"':
                    raw_hashes = k - (j + 1)
                    j = k + 1
                    continue
            if ch == '"':
                en_cadena = True
                j += 1
                continue
            if ch == "'" and j + 2 < m:
                if nxt == '\\' and j + 3 < m and linea[j + 3] == "'":
                    j += 4
                    continue
                if nxt != "'" and linea[j + 2] == "'":
                    j += 3
                    continue
            if ch == '{':
                profundidad += 1
                vio_llave = True
            elif ch == '}':
                profundidad -= 1
                if vio_llave and profundidad <= 0:
                    return i + 1
            elif ch == ';' and not vio_llave and profundidad == 0:
                return i + 1
            j += 1
        i += 1
    return n


def sin_pruebas(texto: str) -> str:
    """El código de producción: omite bloques anotados con `#[cfg(test)]` reales (no en comentarios)."""
    lineas = texto.splitlines()
    salida: list[str] = []
    en_bloque_comentario = 0
    i = 0
    n = len(lineas)
    while i < n:
        linea = lineas[i]
        if en_bloque_comentario == 0 and _CFG_TEST_LINEA.match(linea):
            fin = _consumir_bloque_test(lineas, i)
            salida.extend([''] * (fin - i))
            i = fin
            continue
        salida.append(linea)
        j = 0
        m = len(linea)
        en_cadena = False
        while j < m:
            ch = linea[j]
            nxt = linea[j + 1] if j + 1 < m else ''
            if en_bloque_comentario > 0:
                if ch == '/' and nxt == '*':
                    en_bloque_comentario += 1
                    j += 2
                elif ch == '*' and nxt == '/':
                    en_bloque_comentario -= 1
                    j += 2
                else:
                    j += 1
                continue
            if en_cadena:
                if ch == '\\':
                    j += 2
                    continue
                if ch == '"':
                    en_cadena = False
                j += 1
                continue
            if ch == '/' and nxt == '/':
                break
            if ch == '/' and nxt == '*':
                en_bloque_comentario += 1
                j += 2
                continue
            if ch == '"':
                en_cadena = True
            j += 1
        i += 1
    return '\n'.join(salida)


def buscar(texto: str) -> list[tuple[int, str, str]]:
    return [
        (texto.count('\n', 0, m.start()) + 1, motivo, m.group(0))
        for motivo, patron in PROHIBIDOS.items()
        for m in patron.finditer(texto)
    ]


def autoprueba() -> int:
    """Permanente: el verificador detecta cada control prohibido y el `.Send(`, e ignora los módulos de prueba."""
    for texto in (
        'let c = "chkBitacora";',
        'x("COD_Estado_Detalle_Nuevo")',
        'document.id_Obs_centro',
        'mail.Send()',
        'mail.send()',
        'mail.SEND()',
        'a.Send (',
        sin_pruebas('// #[cfg(test)]\nfn a() { mail.send(); }'),
        sin_pruebas('fn a() {}\n#[cfg(test)]\nmod t {\n    const X: &str = "chkBitacora";\n}\nfn b() { mail.SEND(); }'),
    ):
        assert buscar(texto), texto
    assert not buscar('let c = "chkBitacoraria"; // Save()') and not buscar(sin_pruebas('fn a() {}\n#[cfg(test)]\nconst X: &str = "chkBitacora";'))
    print('AUTOPRUEBA: PASA')
    return 0


def revisar() -> list[str]:
    hallazgos = []
    for carpeta in CARPETAS:
        for ruta in sorted(carpeta.rglob('*')):
            if not ruta.is_file() or ruta.suffix not in SUFIJOS or 'target' in ruta.parts:
                continue
            texto = ruta.read_text(encoding='utf-8', errors='replace')
            if ruta.suffix == '.rs':
                texto = sin_pruebas(texto)
            for linea, motivo, nombre in buscar(texto):
                hallazgos.append(f'{ruta.relative_to(RAIZ)}:{linea}: {motivo}: {nombre}')
    return hallazgos


def main() -> int:
    if '--autoprueba' in sys.argv:
        return autoprueba()
    hallazgos = revisar()
    for h in hallazgos:
        print(h)
    print(f'SOLO_LECTURA: {len(hallazgos)} hallazgos; RESULTADO: {"PASA" if not hallazgos else "FALLA"}')
    return 1 if hallazgos else 0


if __name__ == '__main__':
    sys.exit(main())
