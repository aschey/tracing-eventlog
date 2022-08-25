; // MyEventProvider.mc 

; // This is the header section.


SeverityNames=(
               Informational=0x1:STATUS_SEVERITY_INFORMATIONAL
               Warning=0x2:STATUS_SEVERITY_WARNING
               Error=0x3:STATUS_SEVERITY_ERROR
              )


LanguageNames=(English=0x409:MSG00409)


; // The following are the categories of events.

MessageIdTypedef=WORD

MessageId=0x1
SymbolicName=NETWORK_CATEGORY
Language=English
Network Events
.

MessageId=0x2
SymbolicName=DATABASE_CATEGORY
Language=English
Database Events
.

MessageId=0x3
SymbolicName=UI_CATEGORY
Language=English
UI Events
.


; // The following are the message definitions.

MessageIdTypedef=DWORD

MessageId=0x100
Severity=Error
SymbolicName=MSG_ERROR
Language=English
%1
.

MessageId=0x101
Severity=Warning
SymbolicName=MSG_WARNING
Language=English
%1
.

MessageId=0x102
Severity=Informational
SymbolicName=MSG_INFO
Language=English
%1
.

MessageId=0x103
Severity=Informational
SymbolicName=MSG_DEBUG
Language=English
%1
.

MessageId=0x104
Severity=Informational
SymbolicName=MSG_TRACE
Language=English
%1
.
