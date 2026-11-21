**FREE
// Stage 2, S2.183: DSPLY of float fields (8 and 4 bytes). DSPLY sends
// each value to the message queue GN183Q, and the program reads the text
// back through QSYS2.MESSAGE_QUEUE_INFO and writes it to a stream file
// (DSPLY with no queue ends with status 00333 in a PASE job). Captured
// on PUB400 (tools/golden/native183.txt n183_fltdsply) for the native
// unit NFLTDSP.
ctl-opt dftactgrp(*no);
dcl-s f8 float(8) inz(123);
dcl-s f4 float(4) inz(0.5);
dcl-s g8 float(8) inz(-0.000125);
dcl-s t varchar(200);

dsply(e) f8 'GN183Q';
if %error;
  say('E ' + %char(%status));
endif;
last();
dsply(e) f4 'GN183Q';
if %error;
  say('E ' + %char(%status));
endif;
last();
dsply(e) g8 'GN183Q';
if %error;
  say('E ' + %char(%status));
endif;
last();
*inlr = *on;

dcl-proc last;
  exec sql select message_text into :t
             from qsys2.message_queue_info
            where message_queue_name = 'GN183Q'
            order by message_timestamp desc
            fetch first 1 row only;
  if sqlcode <> 0;
    t = 'SQLCODE ' + %char(sqlcode);
  endif;
  say('[' + t + ']');
end-proc;

dcl-proc say;
  dcl-pi *n;
    text varchar(200) const;
  end-pi;
  dcl-s wbuf varchar(200);
  wbuf = text;
  exec sql call qsys2.ifs_write_utf8(path_name => 'gn1.out', line => :wbuf,
                                     overwrite => 'APPEND', end_of_line => 'LF');
end-proc;
