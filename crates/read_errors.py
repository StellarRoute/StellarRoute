import json
with open('errors2.json', 'r', encoding='utf-16le') as f:
    with open('clean_errors_utf8.txt', 'w', encoding='utf-8') as out:
        for line in f:
            try:
                d = json.loads(line)
                if 'message' in d and 'rendered' in d['message']:
                    out.write(d['message']['rendered'] + '\n')
            except: pass
