**FREE
// Stage 2, S2.102: %LEN as the target of an assignment: a varying field
// cut to 3, made 2 longer again (its old characters come back), set to 0,
// set out of range under MONITOR, and a procedure's own varying field.
// Captured on PUB400 (tools/golden/native102.txt n102_lenset) for the
// native unit NLENSET.
ctl-opt dftactgrp(*no);
dcl-s v varchar(8) inz('abcdef');
dcl-s line varchar(80);
%len(v) = 3;
line = '[' + v + ']';
%len(v) = %len(v) + 2;
line += '[' + v + ']';
%len(v) = 0;
line += '[' + v + ']' + %char(%len(v));
monitor;
  %len(v) = 9;
  line += ' D';
on-error;
  line += ' E ' + %char(%status);
endmon;
line += ' ' + local();
say(line);
*inlr = *on;

dcl-proc local;
  dcl-pi *n varchar(10);
  end-pi;
  dcl-s w varchar(6) inz('xyz');
  %len(w) = 1;
  return w;
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
