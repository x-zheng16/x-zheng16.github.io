# Reads _bibliography/papers.bib and exposes the entries as site.data["xz_pubs"],
# already formatted for the homepage layouts (venue line, author markers, links).
# papers.bib stays the single source for publications; it is regenerated from
# PROFILE.md by the profile CLI.
require "bibtex"
require "cgi"

module XZ
  class PubsGenerator < Jekyll::Generator
    safe true
    priority :high

    SELF = "Xiang Zheng"

    def generate(site)
      path = File.join(site.source, "_bibliography", "papers.bib")
      return unless File.exist?(path)

      bib = BibTeX.open(path, parse_names: false)
      pubs = bib.data.select { |e| e.is_a?(BibTeX::Entry) }.map { |e| build(e) }
      site.data["xz_pubs"] = pubs
      site.data["xz_pub_years"] = pubs.map { |p| p["year"] }.uniq.sort.reverse
    end

    private

    def clean(v)
      v.to_s.gsub("\\&", "&").gsub(/[{}]/, "").strip
    end

    def h(s)
      CGI.escapeHTML(s)
    end

    def build(e)
      f = ->(k) { e.has_field?(k) ? clean(e[k]) : nil }
      journal = f.(:journal)
      type =
        if e.type == :inproceedings then "conference"
        elsif journal.to_s =~ /arxiv/i then "preprint"
        else "journal"
        end
      venue = f.(:abbr).to_s
      venue_full = (f.(:booktitle) || journal).to_s
      year = f.(:year).to_i

      {
        "key" => e.key.to_s,
        "title" => h(f.(:title).to_s),
        "year" => year,
        "type" => type,
        "selected" => f.(:selected) == "true",
        "authors_html" => authors_html(f.(:author).to_s),
        "where" => h(where(type, venue, venue_full, year, f.(:arxiv))),
        "links" => links(type, e, f),
      }
    end

    # "Zheng†, Xiang and Li*, Xiao" -> display names with * (equal) and † (corresponding)
    def authors_html(raw)
      names = raw.split(/\s+and\s+/).map do |a|
        last, first = a.split(",", 2).map(&:strip)
        marks = last.scan(/[*†]/).sort_by { |m| m == "*" ? 0 : 1 }.join
        last = last.delete("*†")
        name = first ? "#{first} #{last}" : last
        [name, marks]
      end
      self_idx = names.index { |n, _| n == SELF } || 0
      limit = [8, self_idx + 1].max
      fmt = names.map { |n, m| n == SELF ? "<b>#{h(n + m)}</b>" : h(n + m) }
      return fmt.join(", ") if names.length <= limit + 2

      shown = fmt.first(limit).join(", ")
      rest = fmt.drop(limit)
      "#{shown}<span class=\"au-x\" hidden>, #{rest.join(', ')}</span>" \
        "<span class=\"au-b\">, <button class=\"morebtn\" type=\"button\">and #{rest.length} more</button></span>"
    end

    def where(type, venue, venue_full, year, arxiv)
      return "arXiv:#{arxiv}" if type == "preprint"

      if type == "conference" && (m = venue.sub(/ Poster$/, "").match(/^([^']+)'(\d{2})\s*(.*)$/))
        conf = "#{m[1]} 20#{m[2]}"
        return conf if m[3].empty? || m[3] == "E&D"
        return "#{conf} #{m[3]}"
      end
      full = type == "journal" && !venue_full.include?("(") ? "#{venue_full} (#{venue})" : venue_full
      full =~ /\d{4}/ ? full : "#{full} #{year}"
    end

    def links(type, e, f)
      l = []
      l << ["arXiv", "https://arxiv.org/abs/#{f.(:arxiv)}"] if f.(:arxiv)
      l << ["doi", "https://doi.org/#{f.(:doi)}"] if f.(:doi) && type != "journal"
      l << ["OpenReview", f.(:openreview)] if f.(:openreview)
      l << ["code", "https://github.com/#{f.(:github)}"] if f.(:github)
      l << ["project", f.(:website)] if f.(:website)
      l.map { |label, url| { "label" => label, "url" => url } }
    end
  end
end
