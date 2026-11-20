# Problemes du Mode Prompt

## "The AI model is not installed yet"

Le fichier du modele integre n'a pas ete installe avec Murmure et n'a pas encore ete telecharge.

**Solution** : ouvrez **Extensions** > **Mode Prompt** > **Configuration avancée** et cliquez sur **Télécharger le modèle** (environ 3 Go, une seule fois). Vous pouvez aussi placer `Qwen_Qwen3.5-4B-Q4_K_M.gguf` a cote du fichier d'installation de Murmure et relancer l'installation.

## "The downloaded model failed its integrity check"

Le telechargement a ete corrompu ou interrompu. Murmure supprime le fichier defectueux. Cliquez a nouveau sur **Télécharger le modèle**.

## "The AI engine did not start in time"

Le moteur dispose de 90 secondes pour charger le modele. Cela peut echouer sur un disque tres lent ou quand l'ordinateur manque de memoire.

**Solution** : fermez les autres applications gourmandes et reessayez. Le moteur a besoin d'environ 3 Go de memoire graphique libre (ou de RAM sans carte graphique).

## "The built-in AI engine is missing from this install"

Les fichiers du moteur ont ete supprimes, souvent par un antivirus. Reinstallez Murmure.

## Le LLM ajoute des guillemets ou des balises

Certains modeles enveloppent leur sortie dans des guillemets (`"..."`) ou ajoutent des balises `<think>...</think>`.

**Solutions :**

1. Utilisez les modeles recommandes : Qwen 3.5, Ministral
2. Ajoutez a votre prompt systeme : "Donne uniquement le resultat. Pas de guillemets, pas de reflexion, pas d'explication."

## Le LLM est tres lent

- **Pas de GPU** : sans carte graphique, l'IA intégrée tourne sur le processeur et est plus lente.
- **Premiere requete** : la premiere requete demarre le moteur et charge le modele (quelques secondes). Les suivantes prennent environ une seconde. Apres 10 minutes sans utilisation, le moteur s'arrete, et la requete suivante redevient une premiere requete.

## Problemes de connexion au serveur

Pour un serveur compatible OpenAI sur une autre machine :

1. Verifiez que le serveur est accessible : `curl http://<serveur>:<port>/v1/models`
2. Verifiez les pare-feu des deux machines
3. L'URL dans Murmure doit inclure le protocole (`http://` ou `https://`)
4. Pour Ollama sur une autre machine, assurez-vous que `OLLAMA_HOST=0.0.0.0` est defini sur le serveur et utilisez `http://<serveur>:11434/v1`

!!! note "Serveur sans endpoint /models"
    Certains serveurs compatibles OpenAI, comme l'API Claude, n'exposent pas `/models`. Le test de connexion echoue alors sur la liste des modeles, mais le serveur reste utilisable : saisissez le nom exact du modele dans le champ modele et le fournisseur OpenAI compatible reste disponible. Une liste de modeles indisponible ne verrouille jamais le fournisseur.

!!! note "Support proxy"
    Le proxy HTTP pour le mode prompt n'est pas encore supporte. Commentez sur [#286](https://github.com/Kieirra/murmure/issues/286) si vous en avez besoin.
