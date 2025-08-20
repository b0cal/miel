## Common Rust application architectures 
#### Tokio + async/await (boucle d’événements)
- _Explication brève_ → Le runtime (Tokio) gère toutes les connexions de manière asynchrone grâce à une boucle d’événements, au lieu d’avoir 1 thread par client
- _Avantages_ → rapide, supporte beaucoup de clients
- _Désavantages_ → demande de comprendre le modèle async/await

#### Actix
- _Explication brève_ → L’app est constituée de plusieurs "acteurs" (petits robots) qui effectuent chacun une tâche précise (ex : gérer un utilisateur, envoyer un message). Ils communiquent entre eux par messages
- _Avantages_ → organisation claire pour les systèmes complexes et concurrents
- _Désavantages_ → framework un peu lourd pour de petits projets

#### ECS
- _Explication brève_ → ECS = "Entity - Component - System"
    - Entité = objet (joueur, ennemi, attaquant)
    - Composant = infos attachées à cet objet (IP, vie, position, etc.)
    - Système = logique qui agit sur tous les objets (déplacement, interaction…)→ Très utilisé dans les jeux vidéos ou les simulations
- _Avantages_ → super efficace pour gérer beaucoup d’objets/composants différents
- _Désavantages_ → pas adapté aux serveurs réseau classiques (HTTP/SMTP/SSH)

#### Serveur multiprotocole (Protocol dispatcher)
- _Explication brève_ → Un même programme peut écouter plusieurs ports (22=SSH, 80=HTTP, etc.) ou partager un port et négocier quel protocole utiliser
- _Avantages_ → centralise plusieurs services dans un seul serveur, mutualisation des ressources
- _Désavantages_ → plus complexe, un plantage fait tomber tous les services, erreurs possibles si on mélange plusieurs protocoles sur un port

## Crates ecosystem options
- tokio
	- Runtime
	- Lien: https://crates.io/crates/tokio
- tokio-rustls
	- Tokio mais sous TLS
	- Lien: https://crates.io/crates/tokio-rustls
- hyper
	- serveur/client HTTP
	- Lien: https://crates.io/crates/hyper
- thrussh
	- Si on veut implémenter notre propre SSH en Rust
	- Lien: https://crates.io/crates/thrussh
- async-smtp
	- client SMTPasyn
	- Lien: https://crates.io/crates/async-smtp
- imap
	- client IMAP
	- Lien: https://crates.io/crates/imap
- bollard
	- Bibliothèque cliente Rust asynchrone pour l'API Docker
	- Lien: https://crates.io/crates/bollard
- bevy
	- moteur de jeu pour l'architecture ECS, donc utile dans le cas ou utiliserait une partie de cette architecture
	- Lien: https://crates.io/crates/bevy
