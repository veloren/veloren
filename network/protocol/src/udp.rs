// TODO: quick and dirty which activly waits for an ack!
/*
UDP protocol

All Good Case:
S --HEADER--> R
S --DATA--> R
S --DATA--> R
S <--FINISHED-- R


Delayed HEADER:
S --HEADER-->
S --DATA--> R // STORE IT
  --HEADER--> R // apply left data and continue
S --DATA--> R
S <--FINISHED-- R


NO HEADER:
S --HEADER--> !
S --DATA--> R // STORE IT
S --DATA--> R // STORE IT
S <--MISSING_HEADER-- R // SEND AFTER 10 ms after DATA1
S --HEADER--> R
S <--FINISHED-- R


NO DATA:
S --HEADER--> R
S --DATA--> R
S --DATA--> !
S --STATUS--> R
S <--MISSING_DATA -- R
S --DATA--> R
S <--FINISHED-- R
*/
