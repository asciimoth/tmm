# tmm
This bot allows you to logically bind two telegram chats so that users who are not in the master cannot be in the slave chat.

## Commands
### getchatid
Returns the ID of the current chat
### bindtochat
Connects the current chat with another by its ID  
`/bindtochat <chatid>`  
One slave chat can have only one master chat  
### unbindfromall
Unbind current chat from the master chat
