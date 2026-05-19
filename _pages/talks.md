---
layout: page
permalink: /talks/
title: Talks
description: Invited talks and seminars.
nav: true
nav_order: 3
---

<div class="publications">

{% assign years = site.data.talks | map: "year" | uniq | sort | reverse %}
{% for yr in years %}
  <h2 class="year">{{ yr }}</h2>
  {% assign yr_talks = site.data.talks | where: "year", yr %}
  {% for talk in yr_talks %}
    <div class="row">
      <div class="col col-sm-2 abbr">
        {% assign venue = site.data.venues[talk.venue_abbr] %}
        {% if venue and venue.color != blank %}
          <abbr class="badge rounded w-100" style="background-color:{{ venue.color }}">
            {% if venue.url %}
              <a href="{{ venue.url }}">{{ talk.venue_abbr }}</a>
            {% else %}
              <div>{{ talk.venue_abbr }}</div>
            {% endif %}
          </abbr>
        {% else %}
          <abbr class="badge rounded w-100">{{ talk.venue_abbr }}</abbr>
        {% endif %}
      </div>
      <div class="col-sm-10">
        <div class="title">{{ talk.title }}</div>
        <div class="author">{{ talk.host }}</div>
        <div class="periodical">
          {{ talk.location }}{% if talk.date != "" %} &middot; {{ talk.date }} {{ yr }}{% endif %}
        </div>
      </div>
    </div>
  {% endfor %}
{% endfor %}

</div>
