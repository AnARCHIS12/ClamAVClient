Placez ici les binaires Linux de ClamAV a embarquer dans l'application.

Recommandation officielle:
- utilisez de preference les packages installateurs officiels ClamAV (.rpm ou .deb)
- la documentation officielle indique que ces builds ont les dependances externes compilees statiquement
- cela les rend plus simples a redistribuer qu'un assemblage depuis des paquets distro

Sources officielles:
- https://docs.clamav.net/manual/Installing.html
- https://www.clamav.net/downloads

Binaires attendus:
- clamscan
- freshclam
- clamdscan (optionnel)
- clamd (optionnel)

Sur Linux, rendez les binaires executables avant le build:
chmod +x clamscan freshclam
chmod +x clamdscan clamd

Note:
- les installateurs officiels Linux s'installent sous /usr/local d'apres la doc ClamAV
- copiez les executables depuis cette installation vers ce dossier de bundle
