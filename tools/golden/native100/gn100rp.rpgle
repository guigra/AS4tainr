**FREE
// Stage 2, S2.99: calls that return a pointer: a procedure of this
// module returning %ADDR(buf) + n (and pointer arithmetic on its result),
// and the C function strstr on null-terminated buffers (a match, and
// *NULL for none). Captured on PUB400 (tools/golden/native100.txt
// n100_retptr) for the native unit NRETPTR.
ctl-opt dftactgrp(*no) bnddir('QC2LE');
dcl-pr strstr pointer extproc('strstr');
  s pointer value;
  t pointer value;
end-pr;
dcl-s buf char(12);
dcl-s key char(3);
dcl-s nokey char(3);
dcl-s p pointer;
dcl-s q pointer;
dcl-s line varchar(80);
buf = 'hello world' + x'00';
key = 'wo' + x'00';
nokey = 'zz' + x'00';
p = at(6);
line = %str(p);
p = at(0) + 2;
line += '|' + %str(p: 3);
q = strstr(%addr(buf): %addr(key));
line += '|' + %str(q);
q = strstr(%addr(buf): %addr(nokey));
if q = *null;
  line += '|null';
endif;
say(line);
*inlr = *on;

dcl-proc at;
  dcl-pi *n pointer;
    n int(10) value;
  end-pi;
  return %addr(buf) + n;
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
