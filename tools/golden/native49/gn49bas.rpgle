**FREE
// Stage 2, S2.48: BASED fields read and written through their pointer:
// a field over another, a data structure over a buffer (character and
// packed subfields), a based local in a procedure, and a *NULL pointer
// used (MONITOR). Captured on PUB400 (tools/golden/native49.txt n49_bas)
// for the native unit NBASED.
ctl-opt dftactgrp(*no);
dcl-s buf char(10) inz('ABCDEFGHIJ');
dcl-s pk char(3) inz(x'12345F');
dcl-s p pointer;
dcl-s q pointer;
dcl-s part char(4) based(p);
dcl-ds over based(q) qualified;
  first char(2);
  amount packed(5:0);
end-ds;
dcl-s raw char(5);
dcl-s n packed(7:0);
p = %addr(buf);
say('PART [' + part + ']');
part = 'wxyz';
say('BUF [' + buf + ']');
p = %addr(buf) + 6;
say('SHIFT [' + part + ']');
raw = 'XY' + pk;
q = %addr(raw);
n = over.amount + 1;
say('FIRST [' + over.first + '] AMOUNT ' + %char(n));
over.amount = 999;
say('RAW HEX ' + %char(%len(raw)) + ' [' + over.first + '] ' + %char(over.amount));
say('LOCAL [' + upper4(%addr(buf)) + ']');
say('AFTER [' + buf + ']');
p = *null;
monitor;
  say('NULL [' + part + ']');
on-error;
  say('NULL ERROR ' + %char(%status));
endmon;
*inlr = *on;
dcl-proc upper4;
  dcl-pi *n char(4);
    at pointer value;
  end-pi;
  dcl-s here char(4) based(at);
  here = 'QRST';
  return here;
end-proc;
// Write one line to gn1.out in the job's current directory (DSPLY under
// `system` goes to QSYSOPR, a queue PUB400 users share).
dcl-proc say;
  dcl-pi *n;
    text varchar(200) const;
  end-pi;
  dcl-s line varchar(200);
  line = text;
  exec sql call qsys2.ifs_write_utf8(path_name => 'gn1.out', line => :line,
                                     overwrite => 'APPEND', end_of_line => 'LF');
end-proc;
