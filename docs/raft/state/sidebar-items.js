initSidebarItems({"enum":[["States","Server states. Followers only respond to requestsfrom other servers. If a follower receives no communication,it becomes a candidate and initiates an election. A candidatethat receives votes from a majority of the full cluster becomesthe new leader. Leaders typically operate until they fail."]],"struct":[["Leader",""],["Persistent","Persistent state on all servers:(Updated on stable storage before responding to RPCs)"],["ServerState","Volatile state on all servers"]]});