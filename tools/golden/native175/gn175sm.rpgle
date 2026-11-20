**FREE
// Stage 2, S2.175: SND-MSG *INFO, to the job log, the program going on.
// (The capture's first version also sent *ESCAPE with %MSG and with text
// in MONITOR groups with ON-EXCP 'CPF9898': IBM i ended the program with
// CPF9898 unmonitored, *ESCAPE going to the caller by default.) Captured on PUB400 (tools/golden/native175.txt n175_sndmsg)
// for the native unit NSNDMSG.
ctl-opt dftactgrp(*no);
dcl-s line varchar(80) inz('start');

snd-msg *info 'just a note';
line += ' info';
say(line);
*inlr = *on;

dcl-proc say;
  dcl-pi *n;
    text varchar(200) const;
  end-pi;
  dcl-s wbuf varchar(200);
  wbuf = text;
  exec sql call qsys2.ifs_write_utf8(path_name => 'gn1.out', line => :wbuf,
                                     overwrite => 'APPEND', end_of_line => 'LF');
end-proc;
