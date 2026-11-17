      * Stage 2, S2.64: a multiple-occurrence data structure. Subfields in
      * each occurrence, %OCCUR read and set, OCCUR with factor 1 and a
      * result field, an occurrence out of range under MONITOR (00122). A
      * literal out of range there is RNF7571 at compile time (first run).
      * Captured on PUB400 (tools/golden/native64.txt n64_occ) for the
      * native unit NOCCUR.
     H DFTACTGRP(*NO)
     D rec             DS                  OCCURS(3)
     D  code                          3A
     D  qty                           5P 0
     D cur             S              5P 0
     D tot             S              7P 0
     D i               S             10I 0
     D line            S             60A
      /free
        for i = 1 to 3;
          %occur(rec) = i;
          code = 'C' + %char(i);
          qty = i * 100;
        endfor;
        %occur(rec) = 2;
        line = 'OCC2 ' + code + ' ' + %char(qty) + ' AT ' + %char(%occur(rec));
        say(line);
      /end-free
     C     1             OCCUR     rec           cur
      /free
        line = 'OCC1 ' + code + ' ' + %char(qty) + ' CUR ' + %char(cur);
        say(line);
        tot = 0;
        for i = 3 downto 1;
          %occur(rec) = i;
          tot += qty;
        endfor;
        line = 'TOTAL ' + %char(tot) + ' AT ' + %char(%occur(rec));
        say(line);
        i = 4;
        monitor;
          %occur(rec) = i;
        on-error;
          line = 'RANGE ' + %char(%status) + ' AT ' + %char(%occur(rec));
          say(line);
        endmon;
        *inlr = *on;
      /end-free
     P say             B
     D say             PI
     D  text                         60A   const
     D txt             S             60A   varying
      /free
        txt = %trimr(text);
        exec sql call qsys2.ifs_write_utf8(path_name => 'gn1.out',
                 line => :txt, overwrite => 'APPEND', end_of_line => 'LF');
      /end-free
     P say             E
