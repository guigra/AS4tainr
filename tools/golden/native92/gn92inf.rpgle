**FREE
// Stage 2, S2.79 review (2026-11-18): a printer file with an INFDS. After
// each WRITE and the CLOSE, the operation, record format, status and the
// printer feedback's current line and page (positions 367 and 369).
// GN92P is tools/golden/native92/gn92p.prtf. Captured on PUB400
// (tools/golden/native92.txt n92_infds) for the native unit NPRTINF.
ctl-opt dftactgrp(*no);
dcl-f gn92p printer infds(inf);
dcl-ds inf;
  f_status *status;
  f_opcode *opcode;
  f_record *record;
  f_line int(5) pos(367);
  f_page int(10) pos(369);
end-ds;
amt = 1234.5;
nm = 'ALPHA';
qty = 7;
write hdr;
show();
write dtl;
show();
close gn92p;
show();
*inlr = *on;

dcl-proc show;
  say(%trim(f_opcode) + '|' + %trim(f_record) + '|' + %char(f_status)
      + '|' + %char(f_line) + '|' + %char(f_page));
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
