      * Stage 2, S2.79 review (2026-11-18): EXCEPT to a program-described
      * printer file. BLK prints a varying subfield with blank after: the
      * subfield's bytes are seen through a character subfield over it.
      * EWR prints a number with an edit word named by a constant; it is
      * printed from a procedure that has its own constant of that name.
      * The program reads its own spooled file back. Captured on PUB400
      * (tools/golden/native92.txt n92_except) for the native unit NEXCPT2.
       ctl-opt dftactgrp(*no);
     FQPRINT    O    F  132        PRINTER
       dcl-c ew '    0.  ';
       dcl-ds d;
         v varchar(5) inz('ABC');
         raw char(7) pos(1);
       end-ds;
       dcl-s amt packed(5:2);
       dcl-s line varchar(132);
       exec sql set option commit = *none;
       amt = 12.5;
       say('B ' + %char(%len(v)) + ' ['
           + %xlate(x'00':'0':%subst(raw:3:5)) + ']');
       except blk;
       say('A ' + %char(%len(v)) + ' ['
           + %xlate(x'00':'0':%subst(raw:3:5)) + ']');
       except ewr;
       local();
       close qprint;
       exec sql declare c cursor for select rtrim(spooled_data)
         from table(systools.spooled_file_data(job_name => '*',
                                               spooled_file_name => 'QPRINT'))
         order by ordinal_position;
       exec sql open c;
       dow 1 = 1;
         exec sql fetch c into :line;
         if sqlcode <> 0;
           leave;
         endif;
         say('|' + line + '|');
       enddo;
       exec sql close c;
       *inlr = *on;
     OQPRINT    E            BLK            1
     O                       V              B    10
     OQPRINT    E            EWR            1
     O                       AMT                 30 EW
       dcl-proc local;
         dcl-c ew '  ,  0.  -';
         except ewr;
       end-proc;
       dcl-proc say;
         dcl-pi *n;
           text varchar(200) const;
         end-pi;
         dcl-s wbuf varchar(200);
         wbuf = text;
         exec sql call qsys2.ifs_write_utf8(path_name => 'gn1.out',
                    line => :wbuf, overwrite => 'APPEND',
                    end_of_line => 'LF');
       end-proc;
