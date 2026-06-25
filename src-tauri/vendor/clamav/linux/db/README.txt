Placez ici une base initiale ClamAV pour Linux.

Fichiers attendus:
- main.cvd ou main.cld
- daily.cvd ou daily.cld
- bytecode.cvd ou bytecode.cld

Au premier lancement, ClamAvClient copiera ces signatures dans son dossier de donnees local
afin que freshclam puisse les mettre a jour sans ecrire dans le bundle en lecture seule.

La doc ClamAV indique qu'une installation de type source ou package officiel utilise
habituellement /usr/local/share/clamav pour la base locale.
