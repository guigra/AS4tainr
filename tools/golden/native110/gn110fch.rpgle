**FREE
// Stage 2, S2.110 (ADR 0008): %FLOAT of character data (a number, the
// float form 1.5E3, a decimal comma, and text that is no number under
// MONITOR), and %ADDR of a float field read back through a based float.
// Captured on PUB400 (tools/golden/native110.txt n110_floatchar) for the
// native unit NFLTCHR.
ctl-opt dftactgrp(*no);
dcl-s f8 float(8);
dcl-s txt varchar(10) inz('2,5');
dcl-s pp pointer;
dcl-s bf float(8) based(pp);
dcl-s line varchar(200);
f8 = %float('12.5');
line = %char(f8);
f8 = %float('1.5E3');
line += ' ' + %char(f8);
f8 = %float(txt);
line += ' ' + %char(f8);
monitor;
  f8 = %float('abc');
  line += ' D';
on-error;
  line += ' E ' + %char(%status);
endmon;
pp = %addr(f8);
line += ' ' + %char(bf);
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
